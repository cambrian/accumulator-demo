use super::state::{Block, Utxo};
use accumulator::group::UnknownOrderGroup;
use accumulator::hash::hash_to_prime;
use accumulator::Accumulator;
use multiqueue::{BroadcastReceiver, BroadcastSender};
use rug::Integer;
use std::clone::Clone;
use std::collections::{HashMap, HashSet};
use std::sync::{Arc, Mutex};
use std::thread;
use uuid::Uuid;

#[derive(Clone, Debug)]
pub struct WitnessRequest {
  pub user_id: usize,
  pub request_id: Uuid,
  pub utxos: Vec<Utxo>,
}

#[derive(Clone, Debug)]
pub struct WitnessResponse<G: UnknownOrderGroup> {
  pub request_id: Uuid,
  pub utxos_with_witnesses: Vec<(Utxo, Accumulator<G>)>,
}

#[derive(Clone, Debug)]
pub struct UserUpdate {
  pub utxos_added: Vec<Utxo>,
  pub utxos_deleted: Vec<Utxo>,
}

#[derive(Clone)]
pub struct Bridge<G: UnknownOrderGroup> {
  bridge_id: usize,
  utxo_set_product: Integer,
  utxo_set_witness: Accumulator<G>,
  block_height: u64,
  user_ids: HashSet<usize>,
}

impl<G: UnknownOrderGroup> Bridge<G> {
  /// Assumes all bridges are online from genesis. We may want to implement syncing later.
  /// Also assumes that bridge/user relationships are fixed in `main`.
  pub fn start(
    bridge_id: usize,
    utxo_set_witness: Accumulator<G>,
    utxo_set_product: Integer,
    block_receiver: BroadcastReceiver<Block<G>>,
    witness_request_receiver: BroadcastReceiver<WitnessRequest>,
    witness_response_senders: HashMap<usize, BroadcastSender<WitnessResponse<G>>>,
    user_update_senders: HashMap<usize, BroadcastSender<UserUpdate>>,
  ) {
    let bridge_ref = Arc::new(Mutex::new(Self {
      bridge_id,
      utxo_set_product,
      utxo_set_witness,
      block_height: 0,
      user_ids: user_update_senders.keys().cloned().collect(),
    }));

    // Block updater thread.
    let bridge = bridge_ref.clone();
    let update_thread = thread::spawn(move || {
      for block in block_receiver {
        bridge.lock().unwrap().update(block, &user_update_senders);
      }
    });

    // Witness request handler.
    let bridge = bridge_ref.clone();
    let witness_thread = thread::spawn(move || {
      for request in witness_request_receiver {
        let bridge = bridge.lock().unwrap();
        let utxos_with_witnesses = bridge.create_membership_witnesses(&request.utxos);
        witness_response_senders[&request.user_id]
          .try_send(WitnessResponse {
            request_id: request.request_id,
            utxos_with_witnesses,
          })
          .unwrap();
      }
    });

    update_thread.join().unwrap();
    witness_thread.join().unwrap();
  }

  fn update(
    &mut self,
    block: Block<G>,
    user_update_senders: &HashMap<usize, BroadcastSender<UserUpdate>>,
  ) {
    // Preserves idempotency if multiple miners are leaders.
    if block.height != self.block_height + 1 {
      return;
    }

    let mut user_updates = HashMap::new();
    for user_id in &self.user_ids {
      user_updates.insert(
        user_id,
        UserUpdate {
          utxos_added: Vec::new(),
          utxos_deleted: Vec::new(),
        },
      );
    }

    let mut untracked_additions = Vec::new();
    let mut untracked_deletions = Vec::new();
    for transaction in block.transactions {
      for (utxo, _witness) in transaction.utxos_spent_with_witnesses {
        if self.user_ids.contains(&utxo.user_id) {
          user_updates
            .get_mut(&utxo.user_id)
            .unwrap()
            .utxos_deleted
            .push(utxo.clone());
          self.utxo_set_product /= hash_to_prime(&utxo);
        } else {
          untracked_deletions.push(hash_to_prime(&utxo));
        }
      }
      for utxo in transaction.utxos_created {
        if self.user_ids.contains(&utxo.user_id) {
          user_updates
            .get_mut(&utxo.user_id)
            .unwrap()
            .utxos_added
            .push(utxo.clone());
          self.utxo_set_product *= hash_to_prime(&utxo);
        } else {
          untracked_additions.push(hash_to_prime(&utxo));
        }
      }
    }

    self.utxo_set_witness = self
      .utxo_set_witness
      .clone()
      .update_membership_witness(
        &block.acc_new,
        &[self.utxo_set_product.clone()],
        &untracked_additions[..],
        &untracked_deletions[..],
      )
      .unwrap();
    self.block_height = block.height;

    println!(
      "Bridge {} received block {}.",
      self.bridge_id, self.block_height
    );

    for (user_id, update) in user_updates {
      user_update_senders[user_id].try_send(update).unwrap();
    }
  }

  /// Generates individual membership witnesses for each given UTXO. See Accumulator::root_factor
  /// and BBF V3 section 4.1.
  fn create_membership_witnesses(&self, utxos: &[Utxo]) -> Vec<(Utxo, Accumulator<G>)> {
    let elems: Vec<Integer> = utxos.iter().map(|u| hash_to_prime(u)).collect();
    let agg_mem_wit = self
      .utxo_set_witness
      .clone()
      .exp_quotient(self.utxo_set_product.clone(), elems.iter().product())
      .unwrap();
    agg_mem_wit.root_factor(&elems, utxos)
  }
}

impl UserUpdate {
  pub fn is_empty(&self) -> bool {
    self.utxos_added.len() == 0 && self.utxos_deleted.len() == 0
  }
}
