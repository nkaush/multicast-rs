mod connection_pool;
mod reliable;
mod member;
mod basic;

use member::{MulticastMemberHandle, MemberStateMessage, MemberStateMessageType};
use crate::{
    message::{NetworkMessageType, PriorityProposalType, PriorityRequestType, UserInput}, 
    Config, NodeId, MessagePriority, MessageId, PriorityMessageType
};

use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};
use priority_queue::PriorityQueue;
use reliable::ReliableMulticast;
use std::collections::HashMap;
use basic::BasicMulticast;
use std::cmp::Reverse;
use tokio::select;

use connection_pool::ConnectionPool;

type MulticastGroup = HashMap<String, MulticastMemberHandle>;
type IncomingChannel = UnboundedReceiver<MemberStateMessage>;

struct QueuedMessage {
    transaction: UserInput,
    vote_count: usize,
    is_deliverable: bool
}

impl QueuedMessage {
    fn new(transaction: UserInput) -> Self {
        Self {
            transaction,
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

pub struct Multicast {
    node_id: NodeId,
    
    pq: PriorityQueue<MessageId, Reverse<MessagePriority>>,
    queued_messages: HashMap<MessageId, QueuedMessage>,
    
    next_local_id: usize,
    next_priority_proposal: usize,

    /// Stores all of the multicast group member thread handles
    reliable_multicast: ReliableMulticast,

    /// Receive handle to take input from the CLI loop
    from_cli: UnboundedReceiver<UserInput>,

    /// Send handle to pass messages to the bank logic thread
    to_bank: UnboundedSender<UserInput>,

    // /// Receive handle for client handling threads to send multicast engine any messages
    // from_clients: UnboundedReceiver<MemberStateMessage>,
    
    /// Hold on to the send handle so we always know we can receive messages
    client_snd_handle: UnboundedSender<MemberStateMessage>,
}

impl Multicast {
    pub async fn new(node_id: NodeId, config: &Config, bank_snd: UnboundedSender<UserInput>) -> (Self, UnboundedSender<UserInput>) {
        let (to_multicast, from_cli) = unbounded_channel();

        let (group, from_clients, client_snd_handle) = ConnectionPool::new(node_id.clone())
            .connect(config)
            .await
            .consume();
        eprintln!("done connecting!");

        let basic = BasicMulticast::new(group, from_clients);
        let reliable_multicast = ReliableMulticast::new(basic);

        let this = Self {
            node_id,
            from_cli,
            reliable_multicast,
            pq: PriorityQueue::new(),
            next_local_id: 0,
            next_priority_proposal: 0,
            queued_messages: HashMap::new(),
            to_bank: bank_snd,
            client_snd_handle
        };

        (this, to_multicast)
    }

    fn get_local_id(&mut self) -> MessageId {
        let local_id = self.next_local_id;
        self.next_local_id += 1;
        
        MessageId {
            original_sender: self.node_id.clone(),
            local_id
        }
    }

    fn get_next_priority(&mut self) -> MessagePriority {
        let priority = self.next_priority_proposal;
        self.next_priority_proposal += 1;
        
        MessagePriority {
            priority,
            proposer: self.node_id.clone()
        }
    }

    fn sync_next_priority(&mut self, other_priority: &MessagePriority) {
        if other_priority.priority > self.next_priority_proposal {
            self.next_priority_proposal = other_priority.priority + 1;
        }
    }

    fn print_pq(&self) {
        for (id, pri) in self.pq.clone().into_sorted_iter() {
            eprint!("({} - {} - pri={} by={}) ", id.original_sender, id.local_id, pri.0.proposer, pri.0.proposer);
        }
        eprintln!("\n");
    }

    fn try_empty_pq(&mut self) {
        while let Some((id, _)) = self.pq.peek() {
            self.print_pq();
            let qm = self.queued_messages.get(id).unwrap();
            if qm.is_deliverable() {
                let qm = self.queued_messages.remove(id).unwrap();
                self.pq.pop();
                self.to_bank.send(qm.transaction).unwrap();
            } else {
                break;
            }
        }
        self.print_pq();
    }

    /// We got some input from the CLI, now we want to request a priority for it.
    fn request_priority(&mut self, msg: UserInput) {
        let local_id = self.get_local_id();
        let my_pri = self.get_next_priority();

        self.pq.push(local_id.clone(), Reverse(my_pri.clone()));
        self.queued_messages.insert(
            local_id.clone(), 
            QueuedMessage::new(msg.clone())
        );
        
        let rq_type = PriorityRequestType { local_id, message: msg };
        self.reliable_multicast.broadcast(NetworkMessageType::PriorityRequest(rq_type));
    }

    /// We got a request from another process for priority, so propose a priority.
    fn propose_priority(&mut self, request: PriorityRequestType) {
        let requester_local_id = request.local_id;
        let priority = self.get_next_priority();
        let recipient = requester_local_id.original_sender.clone();

        self.pq.push(requester_local_id.clone(), Reverse(priority.clone()));
        self.queued_messages.insert(
            requester_local_id.clone(),
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
    fn confirmed_message_priority(&mut self, message_id: MessageId) {
        let agreed_pri = self.pq
            .get_priority(&message_id)
            .cloned()
            .unwrap();
        
        self.reliable_multicast.broadcast(
            NetworkMessageType::PriorityMessage(PriorityMessageType {
                local_id: message_id,
                priority: agreed_pri.0
            }));
    }

    pub async fn main_loop(&mut self) { 
        loop {
            select! {
                input = self.from_cli.recv() => match input {
                    Some(msg) => self.request_priority(msg),
                    None => {
                        println!("from_cli channel closed");
                        break
                    }
                },
                msg = self.reliable_multicast.deliver() => match msg.msg {
                    MemberStateMessageType::Message(net_msg) => {
                        eprintln!("DELIVERED network message from {}: {:?}", msg.member_id, net_msg);
                        match net_msg.msg_type {
                            NetworkMessageType::PriorityRequest(request) => self.propose_priority(request),
                            NetworkMessageType::PriorityProposal(m) => {
                                let mid = m.requester_local_id;
                                let qm = self.queued_messages.get_mut(&mid).unwrap();
                                self.pq.push_increase(mid.clone(), Reverse(m.priority));
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

                                let qm = self.queued_messages.get_mut(&mid).unwrap();
                                self.pq.push_increase(mid, Reverse(m.priority));
                                
                                qm.mark_deliverable();
                                self.try_empty_pq();
                            }
                        }
                    },
                    MemberStateMessageType::NetworkError => {
                        // remove the client from the group if it encounters a network error
                        // TODO: do we need to notify other clients that this client has died?
                        self.reliable_multicast.remove_member(&msg.member_id);
                    },
                    MemberStateMessageType::DuplicateMessage => ()
                }
            }
        }
    }
}