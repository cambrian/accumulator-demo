// TODO: Remove Clippy suppressions.
use super::state::Transaction;
use super::state::Utxo;
use accumulator::group::UnknownOrderGroup;
use accumulator::Accumulator;
use multiqueue::{BroadcastReceiver, BroadcastSender};
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
    init_utxo: Utxo,
    tx_issuance_freq_in_hz: u64,
    witness_request_sender: BroadcastSender<(Vec<Utxo>)>,
    witness_response_receiver: BroadcastReceiver<(Vec<Accumulator<G>>)>,
    tx_sender: BroadcastSender<Transaction<G>>,
  ) {
    let mut utxo_set = HashSet::new();
    utxo_set.insert(init_utxo);
    let user = User { id, utxo_set };

    // TODO: Sample from Exponential distribution instead of fixed interval?
    let sleep_time = time::Duration::from_millis(1000 / tx_issuance_freq_in_hz);
    loop {
      let utxos_to_delete = vec![user.get_input_for_transaction()];
      witness_request_sender
        .try_send(utxos_to_delete.clone())
        .unwrap();
      let witnesses_to_delete = witness_response_receiver.recv().unwrap();
      // Need to clone UTXO in map to get a value instead of a reference.
      let utxo_witnesses_deleted = utxos_to_delete
        .iter()
        .zip(witnesses_to_delete.clone())
        .map(|(x, y)| (x.clone(), y))
        .collect();
      let new_utxo = Utxo { id: Uuid::new_v4() };
      let new_trans = Transaction {
        utxos_added: vec![new_utxo],
        utxos_deleted: utxo_witnesses_deleted,
      };
      // TODO: If this fails, handle gracefully?
      tx_sender.try_send(new_trans).unwrap();
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
