//! Simulation runner.
//! TODO: Add configurability/statistics.
mod simulation;
use accumulator::group::{Rsa2048, UnknownOrderGroup};
use accumulator::hash::hash_to_prime;
use accumulator::util::int;
use accumulator::Accumulator;
use multiqueue::{broadcast_queue, BroadcastReceiver, BroadcastSender};
use rug::Integer;
use simulation::state::Utxo;
use simulation::{Bridge, Miner, User};
use std::collections::HashMap;
use std::thread;
use uuid::Uuid;

const NUM_MINERS: usize = 5;
const NUM_BRIDGES: usize = 5;
const NUM_USERS: usize = 50;

// NOTE: Ensure that sum of USERS_ASSIGNED_TO_BRIDGE is NUM_USERS.
const USERS_ASSIGNED_TO_BRIDGE: [usize; NUM_BRIDGES] = [10; 5];
const BLOCK_TIME_MS: u64 = 1000;

fn new_queue<T: Clone>() -> (BroadcastSender<T>, BroadcastReceiver<T>) {
  broadcast_queue(256)
}

pub fn run_simulation<G: UnknownOrderGroup>() {
  println!("Simulation starting.");
  let mut simulation_threads = Vec::new();
  let (block_sender, block_receiver) = new_queue();
  let (tx_sender, tx_receiver) = new_queue();

  // Initialize genesis user data.
  let mut user_ids = Vec::new();
  let mut user_utxos = Vec::new();
  let mut user_elems = Vec::new();
  let mut user_utxos_product = int(1);
  let mut init_acc = Accumulator::<G>::new();
  for _ in 0..NUM_USERS {
    let user_id = Uuid::new_v4();
    user_ids.push(user_id);
    let user_utxo = Utxo {
      id: Uuid::new_v4(),
      user_id,
    };
    let user_elem = hash_to_prime(&user_utxo);
    init_acc = init_acc.add(&[user_elem.clone()]).0;
    user_utxos_product *= user_elem.clone();
    user_utxos.push(user_utxo);
    user_elems.push(user_elem);
  }

  let mut user_witnesses = Vec::new();
  for user_elem in &user_elems {
    let user_acc = Accumulator::<G>::new();
    let witness_exp = &user_utxos_product / user_elem.clone();
    user_witnesses.push(user_acc.add(&[witness_exp]).0);
  }

  // Initialize miner threads.
  for miner_idx in 0..NUM_MINERS {
    // These clones cannot go inside the thread closure, since the variable being cloned would get
    // swallowed by the move (see below as well).
    let init_acc = init_acc.clone();
    let block_sender = block_sender.clone();
    let block_receiver = block_receiver.add_stream();
    let tx_receiver = tx_receiver.add_stream();
    simulation_threads.push(thread::spawn(move || {
      Miner::<G>::start(
        miner_idx == 0, // Elect first miner as leader.
        init_acc,
        BLOCK_TIME_MS,
        block_sender,
        block_receiver,
        tx_receiver,
      )
    }));
  }

  // Initialize bridge threads.
  let mut user_idx = 0;
  #[allow(clippy::needless_range_loop)]
  for bridge_idx in 0..NUM_BRIDGES {
    let (witness_request_sender, witness_request_receiver) = new_queue();
    let mut witness_response_senders = HashMap::new();
    let mut utxo_update_senders = HashMap::new();

    let num_users_for_bridge = USERS_ASSIGNED_TO_BRIDGE[bridge_idx];
    let user_elem_witnesses: Vec<(Integer, Accumulator<G>)> = user_elems
      [user_idx..user_idx + num_users_for_bridge]
      .iter()
      .zip(user_witnesses[user_idx..user_idx + num_users_for_bridge].iter())
      .map(|(elem, witness)| (elem.clone(), witness.clone()))
      .collect();
    let bridge_init_acc = init_acc.clone().delete(&user_elem_witnesses).unwrap().0;
    let bridge_utxo_set_product = user_elems[user_idx..user_idx + num_users_for_bridge]
      .iter()
      .product();;

    // Initialize configurable user threads per bridge.
    for _ in 0..num_users_for_bridge {
      let user_id = user_ids[user_idx];
      let user_utxo = user_utxos[user_idx].clone();

      // Associate user IDs with RPC response channels.
      let (witness_response_sender, witness_response_receiver) = new_queue();
      let (utxo_update_sender, utxo_update_receiver) = new_queue();
      witness_response_senders.insert(user_id, witness_response_sender);
      utxo_update_senders.insert(user_id, utxo_update_sender);

      let witness_request_sender = witness_request_sender.clone();
      let tx_sender = tx_sender.clone();
      simulation_threads.push(thread::spawn(move || {
        User::start(
          user_id,
          user_utxo,
          witness_request_sender,
          witness_response_receiver,
          utxo_update_receiver,
          tx_sender,
        );
      }));
      user_idx += 1;
    }

    let block_receiver = block_receiver.add_stream();
    simulation_threads.push(thread::spawn(move || {
      Bridge::<G>::start(
        bridge_init_acc,
        bridge_utxo_set_product,
        block_receiver,
        witness_request_receiver,
        witness_response_senders,
        utxo_update_senders,
      );
    }));
  }

  tx_receiver.unsubscribe();
  println!("Simulation running.");
  simulation_threads.push(thread::spawn(move || {
    for block in block_receiver {
      println!("Block");
    }
  }));
  for thread in simulation_threads {
    thread.join().unwrap();
  }
  println!("Simulation exiting.");
}

pub fn main() {
  run_simulation::<Rsa2048>();
}
