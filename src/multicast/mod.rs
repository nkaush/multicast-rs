mod connection_pool;
mod protocol;
mod reliable;
mod config;
mod member;
mod basic;

pub use config::{Config, NodeId, parse_config};

use protocol::{
    NetworkMessage, NetworkMessageType, MessageId, MessagePriority, 
    PriorityMessageType, PriorityRequestType, PriorityProposalType
};
use member::{MulticastMemberHandle, MemberStateMessage, MemberStateMessageType};
use connection_pool::ConnectionPool;
use reliable::ReliableMulticast;

use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};
use std::{collections::HashMap, cmp::Reverse, fmt};
use serde::{Serialize, de::DeserializeOwned};
use log::{trace, error, log_enabled, Level};
use priority_queue::PriorityQueue;
use tokio::select;

type MulticastGroup<M> = HashMap<NodeId, MulticastMemberHandle<M>>;
type IncomingChannel<M> = UnboundedReceiver<MemberStateMessage<M>>;

struct QueuedMessage<M> {
    message: M,
    vote_count: usize,
    is_deliverable: bool
}

impl<M> QueuedMessage<M> {
    fn new(message: M) -> Self {
        Self {
            message,
            vote_count: 0,
            is_deliverable: false
        }
    }

    fn increment_vote_count(&mut self) {
        self.vote_count += 1;
    }

    fn get_vote_count(&self) -> usize {
        self.vote_count
    }

    fn is_deliverable(&self) -> bool {
        self.is_deliverable
    }

    fn mark_deliverable(&mut self) {
        self.is_deliverable = true;
    }
}

pub struct TotalOrderedMulticast<M> {
    node_id: NodeId,
    
    pq: PriorityQueue<MessageId, Reverse<MessagePriority>>,
    queued_messages: HashMap<MessageId, QueuedMessage<M>>,
    
    next_local_id: usize,
    next_priority_proposal: usize,

    /// A reliable multicast client that delivers messages from members to this node
    reliable_multicast: ReliableMulticast<M>,

    /// Receive handle to take input from the CLI loop
    from_cli: UnboundedReceiver<M>,

    /// Send handle to pass messages to the bank logic thread
    to_bank: UnboundedSender<M>,
    
    // /// Hold on to the send handle so we always know we can receive messages
    // client_snd_handle: UnboundedSender<MemberStateMessage>,
}

impl<M> TotalOrderedMulticast<M> {
    pub async fn new(node_id: NodeId, config: &Config, bank_snd: UnboundedSender<M>) -> (Self, UnboundedSender<M>) where M: 'static + Send + Serialize + DeserializeOwned + fmt::Debug {
        let (to_multicast, from_cli) = unbounded_channel();

        let pool = ConnectionPool::new(node_id)
            .connect(config)
            .await;
        trace!("finished connecting to group!");

        let this = Self {
            node_id,
            from_cli,
            reliable_multicast: ReliableMulticast::new(pool.group, pool.from_members),
            pq: PriorityQueue::new(),
            next_local_id: 0,
            next_priority_proposal: 0,
            queued_messages: HashMap::new(),
            to_bank: bank_snd,
            // pool.client_snd_handle
        };

        (this, to_multicast)
    }

    fn get_local_id(&mut self) -> MessageId {
        let local_id = self.next_local_id;
        self.next_local_id += 1;
        
        MessageId {
            original_sender: self.node_id,
            local_id
        }
    }

    fn get_next_priority(&mut self) -> MessagePriority {
        let priority = self.next_priority_proposal;
        self.next_priority_proposal += 1;
        
        MessagePriority {
            priority,
            proposer: self.node_id
        }
    }

    fn sync_next_priority(&mut self, other_priority: &MessagePriority) {
        if other_priority.priority > self.next_priority_proposal {
            self.next_priority_proposal = other_priority.priority + 1;
        }
    }

    fn print_pq(&self) {
        let mut pq_str = String::new();
        for (id, pri) in self.pq.clone().into_sorted_iter() {
            pq_str += format!("(NODE={} ID={} PRI={} BY={}) ", id.original_sender, id.local_id, pri.0.priority, pri.0.proposer).as_str();
        }
        trace!("{}", pq_str);
    }

    fn try_empty_pq(&mut self) where M: fmt::Debug {
        while let Some((id, _)) = self.pq.peek() {
            if log_enabled!(Level::Trace) { self.print_pq(); }

            let qm = self.queued_messages.get(id).unwrap();
            if qm.is_deliverable() {
                let qm = self.queued_messages.remove(id).unwrap();
                self.pq.pop();
                self.to_bank.send(qm.message).unwrap();
            } else {
                break;
            }
        }
        if log_enabled!(Level::Trace) { self.print_pq(); }
    }

    /// We got some input from the CLI, now we want to request a priority for it.
    fn request_priority(&mut self, msg: M) where M: fmt::Debug + Clone {
        let local_id = self.get_local_id();
        let my_pri = self.get_next_priority();

        self.pq.push(local_id, Reverse(my_pri));
        self.queued_messages.insert(
            local_id, 
            QueuedMessage::new(msg.clone())
        );
        
        let rq_type = PriorityRequestType { local_id, message: msg };
        self.reliable_multicast.broadcast(NetworkMessageType::PriorityRequest(rq_type));
    }

    /// We got a request from another process for priority, so propose a priority.
    fn propose_priority(&mut self, request: PriorityRequestType<M>) where M: fmt::Debug + Clone {
        let requester_local_id = request.local_id;
        let priority = self.get_next_priority();
        let recipient = requester_local_id.original_sender;

        self.pq.push(requester_local_id, Reverse(priority));
        self.queued_messages.insert(
            requester_local_id,
            QueuedMessage::new(request.message)
        );

        let proposed_pri = PriorityProposalType {
            requester_local_id,
            priority
        };
        
        let msg_type = NetworkMessageType::PriorityProposal(proposed_pri);
        self.reliable_multicast.send_single(msg_type, &recipient);
    }

    /// We got all the priorities for our message back, so send out the 
    /// confirmed priority along with the message.
    fn confirmed_message_priority(&mut self, message_id: MessageId) where M: fmt::Debug + Clone {
        let priority = self.pq
            .get_priority(&message_id)
            .unwrap().0;
        
        self.reliable_multicast.broadcast(
            NetworkMessageType::PriorityMessage(PriorityMessageType {
                local_id: message_id,
                priority
            }));
    }

    pub async fn main_loop(&mut self) where M: Clone + fmt::Debug { 
        loop {
            select! {
                input = self.from_cli.recv() => match input {
                    Some(msg) => self.request_priority(msg),
                    None => {
                        error!("from_cli channel closed");
                        break
                    }
                },
                msg = self.reliable_multicast.deliver() => match msg.msg {
                    MemberStateMessageType::Message(net_msg) => {
                        trace!("DELIVERED network message from {}: {:?}", msg.member_id, net_msg);
                        match net_msg.msg_type {
                            NetworkMessageType::PriorityRequest(request) => self.propose_priority(request),
                            NetworkMessageType::PriorityProposal(m) => {
                                let mid = m.requester_local_id;
                                let qm = self.queued_messages.get_mut(&mid).unwrap();

                                // We are reversing the priority, so push decrease will be inverted 
                                // and push if the new inner priority is greater than the old priority
                                self.pq.push_decrease(mid, Reverse(m.priority));
                                qm.increment_vote_count();

                                if qm.get_vote_count() >= self.reliable_multicast.size() {
                                    qm.mark_deliverable();
                                    
                                    self.confirmed_message_priority(mid);
                                    self.try_empty_pq();
                                }
                            },
                            NetworkMessageType::PriorityMessage(m) => {
                                let mid = m.local_id;
                                self.sync_next_priority(&m.priority);

                                self.queued_messages
                                    .get_mut(&mid)
                                    .unwrap()
                                    .mark_deliverable();

                                self.pq.push_decrease(mid, Reverse(m.priority));
                                self.try_empty_pq();
                            }
                        }
                    },
                    MemberStateMessageType::NetworkError => {
                        // remove the client from the group if it encounters a network error
                        self.reliable_multicast.remove_member(&msg.member_id);
                    },
                    MemberStateMessageType::DuplicateMessage => ()
                }
            }
        }
    }
}