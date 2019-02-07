// TODO: Remove Clippy suppressions.
use super::state::Transaction;
use super::state::Utxo;
use accumulator::group::UnknownOrderGroup;
use accumulator::Accumulator;
use multiqueue::{BroadcastReceiver, BroadcastSender};
use rand::Rng;
use std::collections::HashSet;
use std::{thread, time};
use uuid::Uuid;

#[allow(dead_code)]
pub struct User {
  id: Uuid, // For bridges to know who to send witness responses to.
  utxo_set: HashSet<Utxo>,
}

#[allow(dead_code)]
impl User {
  #[allow(unused_variables)]
  pub fn launch<G: 'static + UnknownOrderGroup>(
    id: Uuid,
    witness_request_sender: BroadcastSender<(Vec<Utxo>)>,
    witness_response_receiver: BroadcastReceiver<(Vec<Accumulator<G>>)>,
    tx_sender: BroadcastSender<Transaction<G>>,
  ) {
    // Initialize some Utxos so we can sample
    let random_utxo = Utxo { id: Uuid::new_v4() };
    let mut utxo_set = HashSet::new();
    utxo_set.insert(random_utxo);
    let user = User { id, utxo_set };
    let sleep_time = time::Duration::from_millis(1000 / rand::thread_rng().gen_range(1, 11));
    loop {
      let del_utxo = vec![user.get_input_for_transaction()];
      if witness_request_sender.try_send(del_utxo.clone()).is_ok() {
        let witness = witness_response_receiver.recv().unwrap();
        // Need to map to clone utxo to get value, not reference
        let utxos_deleted = del_utxo
          .iter()
          .zip(witness.clone())
          .map(|(x, y)| (x.clone(), y))
          .collect();
        let new_utxo = Utxo { id: Uuid::new_v4() };
        let new_trans = Transaction {
          utxos_added: vec![new_utxo],
          utxos_deleted,
        };
        if tx_sender.try_send(new_trans).is_ok() {
          // TODO: Add some success indication ?
        }
      }
      thread::sleep(sleep_time);
    }
  }

  // TODO: Maybe support more inputs than one.
  // Expects executable to call `update` to remove this UTXO when it is confirmed.
  fn get_input_for_transaction(&self) -> Utxo {
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
