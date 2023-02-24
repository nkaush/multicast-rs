use tokio::sync::mpsc::{UnboundedReceiver};

use super::{MulticastMemberHandle, MemberStateMessage};
use std::collections::HashMap;

pub(super) type MulticastGroup = HashMap<String, MulticastMemberHandle>;
pub(super) type IncomingChannel = UnboundedReceiver<MemberStateMessage>;