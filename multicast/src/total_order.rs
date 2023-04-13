use super::connection_pool::ConnectionPool;
use super::reliable::ReliableMulticast;
use super::{Multicast, MulticastError};
use super::config::{Config, NodeId};
use super::pipe::{UnboundedPipe, unbounded_pipe};

use tokio::sync::mpsc::{UnboundedSender, UnboundedReceiver, unbounded_channel};
use tokio::task::JoinHandle;
use serde::{Serialize, Deserialize, de::DeserializeOwned};
use std::{collections::{HashSet, HashMap}, cmp::Reverse};
use log::{trace, error, log_enabled, Level};
use priority_queue::PriorityQueue;
use async_trait::async_trait;
use tokio::{select, time};
use std::cmp::Ordering;

static MAX_MESSAGE_LATENCY_SECS: u64 = 4;

struct QueuedMessage<M> {
    message: M,
    is_deliverable: bool,
    votes: HashSet<NodeId>
}

impl<M> QueuedMessage<M> {
    fn new(message: M) -> Self {
        Self {
            message,
            is_deliverable: false,
            votes: Default::default()
        }
    }

    fn add_voter(&mut self, voter: NodeId) {
        self.votes.insert(voter);
    }

    fn is_deliverable(&self) -> bool {
        self.is_deliverable
    }

    fn mark_deliverable(&mut self) {
        self.is_deliverable = true;
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum TotalOrderNetworkMessage<M> {
    PriorityRequest(PriorityRequestArgs<M>),
    PriorityProposal(PriorityProposalArgs),
    PriorityMessage(PriorityMessageArgs),
    DirectMessage(M)
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriorityRequestArgs<M> {
    pub local_id: MessageId,
    pub message: M
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriorityProposalArgs {
    pub requester_local_id: MessageId,
    pub priority: MessagePriority
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PriorityMessageArgs {
    pub local_id: MessageId,
    pub priority: MessagePriority,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Hash, PartialEq, Eq)]
pub struct MessageId {
    pub original_sender: NodeId,
    pub local_id: usize
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct MessagePriority {
    pub priority: usize,
    pub proposer: NodeId
}

impl Ord for MessagePriority {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => self.proposer.cmp(&other.proposer),
            x => x
        }
    }
}

impl PartialOrd for MessagePriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        match self.priority.cmp(&other.priority) {
            Ordering::Equal => Some(self.proposer.cmp(&other.proposer)),
            x => Some(x)
        }
    }
}

impl PartialEq for MessagePriority {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.proposer == other.proposer
    }
}

impl Eq for MessagePriority {}

pub struct TotalOrderedMulticast<M> {
    /// Receiver half of the channel to communicate between the handler task and
    /// the deliver API
    deliver_rcv: UnboundedReceiver<M>,
    broadcast_queue: UnboundedPipe<Result<(), MulticastError>, M>,
    send_queue: UnboundedPipe<Result<(), MulticastError>, (M, NodeId)>,
    work_thread_handle: JoinHandle<()>
}

impl<T> Drop for TotalOrderedMulticast<T> {
    fn drop(&mut self) {
        self.work_thread_handle.abort()
    }
}

struct TotalOrderedMulticastWorkData<M> {
    node_id: NodeId,
    
    pq: PriorityQueue<MessageId, Reverse<MessagePriority>>,
    queued_messages: HashMap<MessageId, QueuedMessage<M>>,
    
    next_local_id: usize,
    next_priority_proposal: usize,

    /// A reliable multicast client that delivers messages from members to this node
    reliable_multicast: ReliableMulticast<TotalOrderNetworkMessage<M>>,

    /// Receives message to flush the priority queue of all messages from a dead 
    /// sender after waiting for a particular timeout.
    pq_flush_rcv: UnboundedReceiver<NodeId>,

    /// Send handle for messages to flush the priority queue of all messages 
    /// from a dead sender after waiting for a particular timeout.
    pq_flush_snd: UnboundedSender<NodeId>,

    /// Sender half of the channel to communicate between the handler task and
    /// the deliver API
    deliver_snd: UnboundedSender<M>,

    /// One half of a pipe that receives messages to broadcast and yields the 
    /// result of the broadcast attempt
    broadcast_queue: UnboundedPipe<M, Result<(), MulticastError>>,

    /// One half of a pipe that receives messages to send and yields the 
    /// result of the send attempt
    send_queue: UnboundedPipe<(M, NodeId), Result<(), MulticastError>>,
}

impl<M> TotalOrderedMulticastWorkData<M> {
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
        for (id, pri) in self.pq.clone().into_sorted_iter().take(20) {
            let qm = self.queued_messages.get(&id).unwrap();
            pq_str += format!(
                "(NODE={} ID={} PRI={} BY={} V={:?} D={}) ", 
                id.original_sender, 
                id.local_id, 
                pri.0.priority, 
                pri.0.proposer, 
                qm.votes,
                qm.is_deliverable()
            ).as_str();
        }
        trace!("length={} --> {}", self.pq.len(), pq_str);
    }

    fn flush_pq_unconfirmed_messages(&mut self, member_id: NodeId) {
        if log_enabled!(Level::Trace) { 
            trace!("Flushing PQ of all messages from node {}", member_id);
            self.print_pq(); 
        }
        let to_remove = self.queued_messages
            .iter()
            .filter(|(mid, qm)| {
                mid.original_sender == member_id && !qm.is_deliverable()
            })
            .map(|(mid, _)| mid.clone())
            .collect::<Vec<_>>();

        for mid in to_remove.iter() {
            self.queued_messages.remove(mid);
            self.pq.remove(mid);
        }
        if log_enabled!(Level::Trace) { self.print_pq(); }
    }

    async fn recheck_pq_delivery_status(&mut self) -> Result<(), MulticastError> where M: Serialize + Send {
        let to_confirm = self.queued_messages.iter_mut()
            .filter(|(_, qm)| 
                qm.votes.is_superset(self.reliable_multicast.members())
                    && !qm.is_deliverable()
            )
            .map(|(mid, qm)| {
                qm.mark_deliverable();
                mid.clone()
            })
            .collect::<Vec<_>>();

        for mid in to_confirm.into_iter() {
            self.confirmed_message_priority(mid).await?
        }

        Ok(())
    }

    fn try_empty_pq(&mut self) -> Result<(), MulticastError> {
        while let Some((id, _)) = self.pq.peek() {
            if log_enabled!(Level::Trace) { self.print_pq(); }

            let qm = self.queued_messages.get(id).unwrap();
            if qm.is_deliverable() {
                let qm = self.queued_messages.remove(id).unwrap();
                self.pq.pop();
                self.deliver_snd
                    .send(qm.message)
                    .map_err(|_| MulticastError::InternalError)?;
            } else {
                break;
            }
        }
        if log_enabled!(Level::Trace) { self.print_pq(); }

        Ok(())
    }

    /// Request a priority for this message from all other nodes in the group.
    async fn request_priority(&mut self, msg: M) -> Result<(), MulticastError> where M: Serialize + Send {
        let local_id = self.get_local_id();
        let my_pri = self.get_next_priority();

        self.pq.push(local_id, Reverse(my_pri));
        let queued = QueuedMessage::new(unsafe { std::ptr::read(&msg) });
        self.queued_messages.insert(local_id, queued);
        
        let rq_type = PriorityRequestArgs { local_id, message: msg };
        self.reliable_multicast.broadcast(TotalOrderNetworkMessage::PriorityRequest(rq_type)).await
    }

    /// We got a request from another process for priority, so propose a priority.
    async fn propose_priority(&mut self, request: PriorityRequestArgs<M>) -> Result<(), MulticastError> where M: Serialize + Send {
        trace!("Received priority request for {:?}", request.local_id);
        let requester_local_id = request.local_id;
        let priority = self.get_next_priority();
        let recipient = requester_local_id.original_sender;

        self.pq.push(requester_local_id, Reverse(priority));
        self.queued_messages.insert(
            requester_local_id,
            QueuedMessage::new(request.message)
        );

        let proposed_pri = PriorityProposalArgs {
            requester_local_id,
            priority
        };
        
        let msg_type = TotalOrderNetworkMessage::PriorityProposal(proposed_pri);
        self.reliable_multicast.send_to(msg_type, recipient).await
    }

    async fn process_priority_proposal(&mut self, proposal: PriorityProposalArgs) -> Result<(), MulticastError> where M: Serialize + Send {
        trace!("Received priority proposal for {:?} from Node {}", proposal.requester_local_id, proposal.priority.proposer);
        let mid = proposal.requester_local_id;
        let qm = self.queued_messages.get_mut(&mid).unwrap();

        // We are reversing the priority, so push decrease will be inverted 
        // and push if the new inner priority is greater than the old priority
        self.pq.push_decrease(mid, Reverse(proposal.priority));
        qm.add_voter(proposal.priority.proposer);

        if qm.votes.is_superset(self.reliable_multicast.members()) {
            trace!("Received enough votes for delivery for {:?}", proposal.requester_local_id);
            qm.mark_deliverable();
            
            self.confirmed_message_priority(mid).await?;
            self.try_empty_pq()?;
        }

        Ok(())
    }

    /// We got all the priorities for our message back, so send out the 
    /// confirmed priority along with the message.
    async fn confirmed_message_priority(&mut self, message_id: MessageId) -> Result<(), MulticastError> where M: Serialize + Send {
        let priority = self.pq
            .get_priority(&message_id)
            .unwrap().0;
        
        self.reliable_multicast.broadcast(
            TotalOrderNetworkMessage::PriorityMessage(PriorityMessageArgs {
                local_id: message_id,
                priority
            })).await
    }

    fn remove_node(&mut self, node_id: NodeId) {
        self.reliable_multicast.remove_member(&node_id);

        let pq_flush_snd_clone = self.pq_flush_snd.clone();
        tokio::spawn(async move {
            trace!("Waiting for {}s before flushing PQ of all messages from node {}...", MAX_MESSAGE_LATENCY_SECS, node_id);
            time::sleep(time::Duration::from_secs(MAX_MESSAGE_LATENCY_SECS)).await;
            trace!("Waiting for messages from node {} to trickle in finished...", node_id);
            pq_flush_snd_clone.send(node_id).unwrap();
        });
    }

    async fn handle_failure(&mut self, failure: MulticastError) where M: Send + Serialize {
        use MulticastError::*;
        match failure {
            BroadcastError(failures) => {
                failures
                    .into_iter()
                    .for_each(|node_id| self.remove_node(node_id));

                self.recheck_pq_delivery_status().await;
                self.try_empty_pq();
            },
            ClientDisconnected(node_id) => {
                self.remove_node(node_id);
                self.recheck_pq_delivery_status().await;
                self.try_empty_pq();
            },
            AllClientsDisconnected => todo!(), // TODO figure out what to do when all clients disconnect
            InvalidRecipient(_) => unreachable!(),
            InternalError => todo!()
        }
    }
}

async fn to_protocol_loop<M>(mut data: TotalOrderedMulticastWorkData<M>) where M: Serialize + Send { 
    loop {
        select! {
            Some(broadcast_req) = data.broadcast_queue.recv() => {
                let resp = data.request_priority(broadcast_req).await;
                data.broadcast_queue.send(resp).unwrap();
            },
            Some((msg, recipient)) = data.send_queue.recv() => {
                let resp = data
                    .reliable_multicast
                    .send_to(TotalOrderNetworkMessage::DirectMessage(msg), recipient)
                    .await;
                data.send_queue.send(resp).unwrap();
            },
            delivery = data.reliable_multicast.deliver() => match delivery {
                Ok(msg) => match msg {
                    TotalOrderNetworkMessage::PriorityRequest(request) => {
                        if let Err(e) = data.propose_priority(request).await {
                            error!("Failed to propose priority: {:?}", e)
                        }
                    },
                    TotalOrderNetworkMessage::PriorityProposal(proposal) => {
                        if let Err(e) = data.process_priority_proposal(proposal).await {
                            error!("Failed to process priority proposal: {:?}", e)
                        }
                    },
                    TotalOrderNetworkMessage::PriorityMessage(m) => {
                        let mid = m.local_id;
                        data.sync_next_priority(&m.priority);

                        match data.queued_messages.get_mut(&mid) {
                            Some(qm) => {
                                qm.mark_deliverable();
                                data.pq.push_decrease(mid, Reverse(m.priority));
                                data.try_empty_pq();
                            },
                            None => error!("Attempt to retrieve message with id = {:?} from queued_messages failed", mid)
                        }
                    },
                    TotalOrderNetworkMessage::DirectMessage(m) => { data.deliver_snd.send(m); }
                },
                Err(failure) => data.handle_failure(failure).await
            },
            Some(member_id) = data.pq_flush_rcv.recv() => {
                data.flush_pq_unconfirmed_messages(member_id);
                data.try_empty_pq();
            }
        }
    }
}

#[async_trait]
impl<M> Multicast<M> for TotalOrderedMulticast<M> where M: Send + Serialize {
    async fn connect(node_id: usize, config: Config, timeout_secs: u64) -> Self where M: 'static + Serialize + Send + DeserializeOwned { 
        let (pq_flush_snd, pq_flush_rcv) = unbounded_channel();
        let (deliver_snd, deliver_rcv) = unbounded_channel();
        let (broadcast_queue_snd, broadcast_queue_rcv) = unbounded_pipe();
        let (send_queue_snd, send_queue_rcv) = unbounded_pipe();

        let pool = ConnectionPool::new(node_id)
            .with_timeout(timeout_secs)
            .connect(&config)
            .await;

        trace!("finished connecting to group!");

        let data: TotalOrderedMulticastWorkData<M> = TotalOrderedMulticastWorkData {
            node_id,
            reliable_multicast: ReliableMulticast::new(pool.group, pool.from_members),
            pq: PriorityQueue::new(),
            next_local_id: 0,
            next_priority_proposal: 0,
            queued_messages: HashMap::new(),
            pq_flush_rcv,
            pq_flush_snd,
            deliver_snd,
            broadcast_queue: broadcast_queue_rcv,
            send_queue: send_queue_rcv
        };

        TotalOrderedMulticast {
            deliver_rcv,
            broadcast_queue: broadcast_queue_snd,
            send_queue: send_queue_snd,
            work_thread_handle: tokio::spawn(to_protocol_loop(data))
        }
    }

    async fn broadcast(&mut self, msg: M) -> Result<(), MulticastError> where M: Serialize { 
        self.broadcast_queue.send(msg).unwrap();
        self.broadcast_queue.recv().await.unwrap()
    }

    async fn send_to(&mut self, msg: M, recipient: NodeId) -> Result<(), MulticastError> where M: Serialize { 
        self.send_queue.send((msg, recipient)).unwrap();
        self.broadcast_queue.recv().await.unwrap()
    }

    async fn deliver(&mut self) -> Result<M, MulticastError> where M: Send + Serialize { 
        match self.deliver_rcv.recv().await {
            Some(msg) => Ok(msg),
            None => unreachable!()
        }
    }
}