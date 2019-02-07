// TODO: Remove Clippy suppressions.
use super::state::{Block, Transaction};
use super::util;
use accumulator::group::UnknownOrderGroup;
use accumulator::Accumulator;
use crossbeam::thread;
use multiqueue::{BroadcastReceiver, BroadcastSender};
use rug::Integer;
use std::sync::Mutex;
use std::thread::sleep;
use std::time::Duration;

#[allow(dead_code)]
pub struct Miner<G: UnknownOrderGroup> {
  acc: Accumulator<G>,
  block_height: u64,
  pending_transactions: Vec<Transaction<G>>,
}

#[allow(dead_code)]
impl<G: UnknownOrderGroup> Miner<G> {
  /// Assumes all miners are online from genesis. We may want to implement syncing later.
  #[allow(unused_variables)]
  pub fn launch(
    is_leader: bool,
    block_interval_seconds: u64,
    block_sender: BroadcastSender<Block<G>>,
    block_receiver: BroadcastReceiver<Block<G>>,
    tx_receiver: BroadcastReceiver<Transaction<G>>,
  ) {
    let miner_lock = Mutex::new(Miner {
      acc: Accumulator::<G>::new(),
      block_height: 0,
      pending_transactions: Vec::new(),
    });

    let block_sender_lock = Mutex::new(block_sender);
    let block_receiver_lock = Mutex::new(block_receiver);
    let tx_receiver_lock = Mutex::new(tx_receiver);

    thread::scope(|s| {
      // Transaction processor thread.
      s.spawn(|_| loop {
        let tx = {
          let tx_receiver = tx_receiver_lock.lock().unwrap();
          tx_receiver.recv().unwrap()
        };
        let mut miner = miner_lock.lock().unwrap();
        (*miner).add_transaction(tx);
      });

      // Block processor thread.
      s.spawn(|_| loop {
        let block = {
          let block_receiver = block_receiver_lock.lock().unwrap();
          (*block_receiver).recv().unwrap()
        };
        let mut miner = miner_lock.lock().unwrap();
        (*miner).validate_block(block);
      });

      // Block creation on an interval.
      if is_leader {
        loop {
          sleep(Duration::from_secs(block_interval_seconds));
          let new_block = {
            let miner = miner_lock.lock().unwrap();
            (*miner).forge_block()
          };
          let block_sender = block_sender_lock.lock().unwrap();
          (*block_sender).try_send(new_block).unwrap();
          // Note: This miner will consume the forged block via validate.
        }
      }
    })
    .unwrap();
  }

  fn add_transaction(&mut self, transaction: Transaction<G>) {
    // This contains check could incur overhead; ideally we'd use a set but Rust HashSet is kind of
    // a pain to use here.
    if !self.pending_transactions.contains(&transaction) {
      self.pending_transactions.push(transaction);
    }
  }

  fn forge_block(&self) -> Block<G> {
    let (elems_added, elems_deleted) = util::elems_from_transactions(&self.pending_transactions);
    let (witness_deleted, proof_deleted) = self.acc.clone().delete(&elems_deleted).unwrap();
    let (new_acc, proof_added) = witness_deleted.clone().add(&elems_added);
    Block {
      height: self.block_height + 1,
      transactions: self.pending_transactions.clone(),
      new_acc,
      proof_added,
      proof_deleted,
    }
  }

  fn validate_block(&mut self, block: Block<G>) {
    // Preserves idempotency if multiple miners are leaders.
    if block.height != self.block_height + 1 {
      return;
    }

    let (elems_added, elem_witnesses_deleted) = util::elems_from_transactions(&block.transactions);
    let elems_deleted: Vec<Integer> = elem_witnesses_deleted
      .iter()
      .map(|(u, _wit)| u.clone())
      .collect();
    assert!(self
      .acc
      .verify_membership(&elems_deleted, &block.proof_deleted));
    assert!(block
      .new_acc
      .verify_membership(&elems_added, &block.proof_added));
    assert!(block.proof_deleted.witness == block.proof_added.witness);
    self.acc = block.new_acc;
    self.block_height = block.height;
    self.pending_transactions.clear();
  }
}
