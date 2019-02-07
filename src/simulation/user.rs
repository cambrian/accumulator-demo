// TODO: Remove Clippy suppressions.
use super::state::Transaction;
use super::state::Utxo;
use accumulator::group::UnknownOrderGroup;
use multiqueue::{BroadcastReceiver, BroadcastSender};
use std::collections::HashSet;
use uuid::Uuid;

#[allow(dead_code)]
pub struct User {
  id: Uuid, // For bridges to know who to send witness responses to.
  utxo_set: HashSet<Utxo>,
}

#[allow(dead_code)]
impl User {
  #[allow(unused_variables)]
  pub fn launch<G: UnknownOrderGroup>(
    id: Uuid,
    witness_request_sender: BroadcastSender<()>, // TODO (type)
    witness_response_receiver: BroadcastReceiver<()>, // TODO (type)
    tx_sender: BroadcastSender<Transaction<G>>,
  ) {
    let user = User {
      id,
      utxo_set: HashSet::new(),
    };
    // TODO
    unimplemented!();
  }

  // TODO: Maybe support more inputs than one.
  // Expects executable to call `update` to remove this UTXO when it is confirmed.
  fn get_input_for_transaction(&mut self) -> Utxo {
    self.utxo_set.iter().next().unwrap().clone()
  }

  fn update(&mut self, deleted_inputs: &[Utxo], added_outputs: &[Utxo]) {
    for del in deleted_inputs {
      self.utxo_set.remove(&del);
    }
    for add in added_outputs {
      self.utxo_set.insert(add.clone());
    }
  }
}
