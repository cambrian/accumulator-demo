use super::state::Transaction;
use super::state::Utxo;
use crate::simulation::bridge::{WitnessRequest, WitnessResponse};
use accumulator::group::UnknownOrderGroup;
use multiqueue::{BroadcastReceiver, BroadcastSender};
use std::collections::HashSet;
use uuid::Uuid;

pub struct User {
  id: Uuid, // For bridges to know who to send witness responses to.
  utxo_set: HashSet<Utxo>,
}

impl User {
  #[allow(unused_variables)]
  // Right now users are limited to one transaction per block (i.e. they can issue one transaction
  // based on their UTXO set as of some block). TODO: Allow for more tx per user per block.
  pub fn start<G: 'static + UnknownOrderGroup>(
    id: Uuid,
    init_utxo: Utxo,
    witness_request_sender: BroadcastSender<WitnessRequest>,
    witness_response_receiver: BroadcastReceiver<WitnessResponse<G>>,
    utxo_update_receiver: BroadcastReceiver<(Vec<Utxo>, Vec<Utxo>)>,
    tx_sender: BroadcastSender<Transaction<G>>,
  ) {
    let mut utxo_set = HashSet::new();
    utxo_set.insert(init_utxo);
    let mut user = User { id, utxo_set };

    loop {
      let mut utxos_to_delete = Vec::new();
      utxos_to_delete.push(user.get_input_for_transaction());

      let witnesses_to_delete = {
        let witness_request_id = Uuid::new_v4();
        loop {
          witness_request_sender
            .try_send(WitnessRequest {
              user_id: user.id,
              request_id: witness_request_id,
              utxos: utxos_to_delete.clone(),
            })
            .unwrap();
          let response = witness_response_receiver.recv().unwrap();
          if response.request_id == witness_request_id {
            break response.witnesses;
          }
          // Drain any other responses so we don't loop forever.
          loop {
            if witness_response_receiver.try_recv().is_err() {
              break;
            }
          }
        }
      };

      let new_utxo = Utxo {
        id: Uuid::new_v4(),
        user_id: user.id,
      };

      let new_trans = Transaction {
        utxos_added: vec![new_utxo],
        utxos_deleted: witnesses_to_delete,
      };

      tx_sender.try_send(new_trans).unwrap();
      let (deleted_inputs, added_outputs) = utxo_update_receiver.recv().unwrap();
      user.update(&deleted_inputs, &added_outputs);
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
