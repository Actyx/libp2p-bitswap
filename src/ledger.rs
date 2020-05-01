use crate::block::Block;
use crate::message::{BitswapMessage, Priority};
use libipld_core::cid::Cid;
use std::collections::HashMap;

/// The Ledger contains the history of transactions with a peer.
#[derive(Debug, Default)]
pub struct Ledger {
    /// The list of wanted blocks sent to the peer.
    sent_want_list: HashMap<Cid, Priority>,
    /// The list of wanted blocks received from the peer.
    received_want_list: HashMap<Cid, Priority>,
    /// Queued message.
    message: BitswapMessage,
}

impl Ledger {
    /// Creates a new `PeerLedger`.
    pub fn new() -> Self {
        Self::default()
    }

    pub fn peer_wants(&self, cid: &Cid) -> bool {
        self.received_want_list.contains_key(cid)
    }

    pub fn add_block(&mut self, block: Block) {
        self.message.add_block(block);
    }

    pub fn want(&mut self, cid: &Cid, priority: Priority) {
        self.message.want_block(cid, priority);
    }

    pub fn cancel(&mut self, cid: &Cid) {
        self.message.cancel_block(cid);
    }

    pub fn send(&mut self) -> Option<BitswapMessage> {
        if self.message.is_empty() {
            return None;
        }
        for cid in self.message.cancel() {
            self.sent_want_list.remove(cid);
        }
        for (cid, priority) in self.message.want() {
            self.sent_want_list.insert(cid.clone(), priority);
        }
        Some(core::mem::replace(&mut self.message, BitswapMessage::new()))
    }

    pub fn receive(&mut self, message: &BitswapMessage) {
        for cid in message.cancel() {
            self.received_want_list.remove(cid);
            self.message.remove_block(cid);
        }
        for (cid, priority) in message.want() {
            self.received_want_list.insert(cid.to_owned(), priority);
        }
    }

    pub fn wantlist<'a>(&'a self) -> impl Iterator<Item = (&Cid, Priority)> + 'a {
        self.received_want_list
            .iter()
            .map(|(cid, priority)| (cid, *priority))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::block::tests::create_block;

    #[test]
    fn test_ledger_send_block() {
        let block_1 = create_block(b"1");
        let block_2 = create_block(b"2");
        let mut ledger = Ledger::new();
        ledger.add_block(block_1);
        ledger.add_block(block_2);
        let message = ledger.send().unwrap();
        assert_eq!(message.blocks().len(), 2);
    }

    #[test]
    fn test_ledger_remove_block() {
        let block_1 = create_block(b"1");
        let block_2 = create_block(b"2");
        let mut ledger = Ledger::new();
        ledger.add_block(block_1.clone());
        ledger.add_block(block_2);

        let mut cancel = BitswapMessage::new();
        cancel.cancel_block(block_1.cid());
        ledger.receive(&cancel);
        let message = ledger.send().unwrap();
        assert_eq!(message.blocks().len(), 1);
    }

    #[test]
    fn test_ledger_send_want() {
        let block_1 = create_block(b"1");
        let block_2 = create_block(b"2");
        let mut ledger = Ledger::new();
        ledger.want(&block_1.cid(), 1);
        ledger.want(&block_2.cid(), 1);
        ledger.cancel(&block_1.cid());
        let message = ledger.send().unwrap();
        let mut want_list = message.want();
        assert_eq!(want_list.next(), Some((block_2.cid(), 1)));
    }

    #[test]
    fn test_ledger_send_cancel() {
        let block_1 = create_block(b"1");
        let block_2 = create_block(b"2");
        let mut ledger = Ledger::new();
        ledger.want(&block_1.cid(), 1);
        ledger.want(&block_2.cid(), 1);
        ledger.send();
        ledger.cancel(&block_1.cid());
        ledger.send();
        let mut want_list = HashMap::new();
        want_list.insert(block_2.cid().clone(), 1);
        assert_eq!(ledger.sent_want_list, want_list);
    }

    #[test]
    fn test_ledger_receive() {
        let block_1 = create_block(b"1");
        let mut message = BitswapMessage::new();
        message.want_block(&block_1.cid().clone(), 1);

        let mut ledger = Ledger::new();
        ledger.receive(&message);

        assert_eq!(ledger.wantlist().next(), Some((block_1.cid(), 1)));

        let mut message = BitswapMessage::new();
        message.cancel_block(&block_1.cid());
        ledger.receive(&message);
        assert_eq!(ledger.wantlist().next(), None);
    }
}
