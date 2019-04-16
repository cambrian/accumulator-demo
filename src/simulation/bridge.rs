use super::state::{Block, Utxo};
use accumulator::group::UnknownOrderGroup;
use accumulator::Witness;
use multiqueue::{BroadcastReceiver, BroadcastSender};
use std::clone::Clone;
use std::collections::{HashMap, HashSet};
use std::hash::Hash;
use std::sync::{Arc, Mutex};
use std::thread;
use uuid::Uuid;

#[derive(Clone, Debug)]
/// A request from a user for a witness stored on this bridge.
pub struct WitnessRequest {
  pub user_id: usize,
  pub request_id: Uuid,
  pub utxos: Vec<Utxo>,
}

#[derive(Clone, Debug)]
/// A response for a particular witness request.
pub struct WitnessResponse<G: UnknownOrderGroup, T: Clone + Hash> {
  pub request_id: Uuid,
  pub utxos_with_witnesses: Vec<(Utxo, Witness<G, T>)>,
}

#[derive(Clone, Debug)]
/// An update to the set of UTXOs tracked by a user (e.g. when a block is received by a bridge).
pub struct UserUpdate {
  pub utxos_added: Vec<Utxo>,
  pub utxos_deleted: Vec<Utxo>,
}

#[derive(Clone)]
/// A bridge node in our system, managing UTXO witnesses for a set of users.
pub struct Bridge<G: UnknownOrderGroup> {
  bridge_id: usize,
  utxo_set: Vec<Utxo>,
  utxo_set_witness: Witness<G, Utxo>,
  block_height: u64,
  user_ids: HashSet<usize>,
}

impl<G: UnknownOrderGroup> Bridge<G> {
  /// Runs a bridge node's simulation loop.
  // Assumes all bridges are online from genesis. We may want to implement syncing later.
  // Also assumes that bridge/user relationships are fixed in `main`.
  pub fn start(
    bridge_id: usize,
    utxo_set_witness: Witness<G, Utxo>,
    utxo_set: Vec<Utxo>,
    block_receiver: BroadcastReceiver<Block<G, Utxo>>,
    witness_request_receiver: BroadcastReceiver<WitnessRequest>,
    witness_response_senders: HashMap<usize, BroadcastSender<WitnessResponse<G, Utxo>>>,
    user_update_senders: HashMap<usize, BroadcastSender<UserUpdate>>,
  ) {
    let bridge_ref = Arc::new(Mutex::new(Self {
      bridge_id,
      utxo_set,
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

  /// Given a new block, updates the witnesses stored on this bridge and propagates UTXO changes to
  /// individual users.
  fn update(
    &mut self,
    block: Block<G, Utxo>,
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
          self.utxo_set.retain(|x| *x != utxo);
        } else {
          untracked_deletions.push(utxo);
        }
      }
      for utxo in transaction.utxos_created {
        if self.user_ids.contains(&utxo.user_id) {
          user_updates
            .get_mut(&utxo.user_id)
            .unwrap()
            .utxos_added
            .push(utxo.clone());
          self.utxo_set.push(utxo);
        } else {
          untracked_additions.push(utxo);
        }
      }
    }

    self.utxo_set_witness = block
      .acc_new
      .update_membership_witness(
        self.utxo_set_witness.clone(),
        &self.utxo_set,
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

  /// Generates individual membership witnesses for each given UTXO. See `Witness::root_factor`
  /// and BBF V3 Section 4.1.
  fn create_membership_witnesses(&self, utxos: &[Utxo]) -> Vec<(Utxo, Witness<G, Utxo>)> {
    let agg_mem_wit = self
      .utxo_set_witness
      .clone()
      .compute_subset_witness(&self.utxo_set, utxos)
      .unwrap();
    agg_mem_wit.compute_individual_witnesses(utxos)
  }
}

impl UserUpdate {
  pub fn is_empty(&self) -> bool {
    self.utxos_added.len() == 0 && self.utxos_deleted.len() == 0
  }
}
