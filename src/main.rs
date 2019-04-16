//! Simulation runner.
mod simulation;
use accumulator::group::{Rsa2048, UnknownOrderGroup};
use accumulator::{Accumulator, Witness};
use multiqueue::{broadcast_queue, BroadcastReceiver, BroadcastSender};
use simulation::state::Utxo;
use simulation::{Bridge, Miner, User};
use std::collections::HashMap;
use std::thread;
use std::thread::sleep;
use std::time::Duration;
use uuid::Uuid;

const NUM_MINERS: usize = 5;
const NUM_BRIDGES: usize = 5;
const NUM_USERS: usize = 15;

// NOTE: Ensure that sum of USERS_ASSIGNED_TO_BRIDGE is NUM_USERS.
const USERS_ASSIGNED_TO_BRIDGE: [usize; NUM_BRIDGES] = [3; 5];
const BLOCK_TIME_MS: u64 = 5000;

fn new_queue<T: Clone>() -> (BroadcastSender<T>, BroadcastReceiver<T>) {
  broadcast_queue(256)
}

pub fn run_simulation<G: UnknownOrderGroup>() {
  println!("Simulation starting.");
  let mut simulation_threads = Vec::new();
  let (block_sender, block_receiver) = new_queue();
  let (tx_sender, tx_receiver) = new_queue();

  // Initialize genesis user data (each user has a single UTXO).
  let mut user_utxos = Vec::new();
  for user_id in 0..NUM_USERS {
    let user_utxo = Utxo {
      id: Uuid::new_v4(),
      user_id,
    };
    user_utxos.push(user_utxo);
  }

  let mut init_acc = Accumulator::<G, Utxo>::empty();
  init_acc = init_acc.add(&user_utxos);

  // Compute initial user witnesses.
  let mut user_witnesses = Vec::new();
  let witness_all = Witness(Accumulator::<G, Utxo>::empty());
  for user_utxo in &user_utxos {
    let user_witness = witness_all
      .clone()
      .compute_subset_witness(&user_utxos, &[user_utxo.clone()])
      .unwrap();
    user_witnesses.push(user_witness);
  }

  // Initialize bridge threads, each of which manages witnesses for a number of users.
  let mut user_idx = 0;
  #[allow(clippy::needless_range_loop)]
  for bridge_idx in 0..NUM_BRIDGES {
    let (witness_request_sender, witness_request_receiver) = new_queue();
    let mut witness_response_senders = HashMap::new();
    let mut utxo_update_senders = HashMap::new();

    let num_users_for_bridge = USERS_ASSIGNED_TO_BRIDGE[bridge_idx];
    let user_elem_witnesses: Vec<(Utxo, Witness<G, Utxo>)> = user_utxos
      [user_idx..user_idx + num_users_for_bridge]
      .iter()
      .zip(user_witnesses[user_idx..user_idx + num_users_for_bridge].iter())
      .map(|(elem, witness)| (elem.clone(), witness.clone()))
      .collect();
    let bridge_init_witness = Witness(init_acc.clone().delete(&user_elem_witnesses).unwrap());
    let bridge_utxo_set: Vec<Utxo> = user_utxos[user_idx..user_idx + num_users_for_bridge].to_vec();

    // Initialize configurable user threads per bridge.
    for _ in 0..num_users_for_bridge {
      let user_utxo = user_utxos[user_idx].clone();

      // Associate user IDs with RPC response channels.
      let (witness_response_sender, witness_response_receiver) = new_queue();
      let (utxo_update_sender, utxo_update_receiver) = new_queue();
      witness_response_senders.insert(user_idx, witness_response_sender);
      utxo_update_senders.insert(user_idx, utxo_update_sender);

      let witness_request_sender = witness_request_sender.clone();
      let tx_sender = tx_sender.clone();
      simulation_threads.push(thread::spawn(move || {
        User::start(
          user_idx,
          bridge_idx,
          user_utxo,
          &witness_request_sender,
          &witness_response_receiver,
          &utxo_update_receiver,
          &tx_sender,
        );
      }));
      user_idx += 1;
    }

    let block_receiver = block_receiver.add_stream();
    simulation_threads.push(thread::spawn(move || {
      Bridge::<G>::start(
        bridge_idx,
        bridge_init_witness,
        bridge_utxo_set,
        block_receiver,
        witness_request_receiver,
        witness_response_senders,
        utxo_update_senders,
      );
    }));
  }

  println!("Sleeping so bridges can start up before miner.");
  sleep(Duration::from_millis(2000));

  // Initialize miner threads.
  for miner_idx in 0..NUM_MINERS {
    // These clones cannot go inside the thread closure, since the variable being cloned would get
    // swallowed by the move (see below as well).
    let init_acc = init_acc.clone();
    let block_sender = block_sender.clone();
    let block_receiver = block_receiver.add_stream();
    let tx_receiver = tx_receiver.add_stream();
    simulation_threads.push(thread::spawn(move || {
      Miner::<G, Utxo>::start(
        miner_idx == 0, // Elect first miner as leader.
        init_acc,
        BLOCK_TIME_MS,
        &block_sender,
        block_receiver,
        tx_receiver,
      )
    }));
  }

  tx_receiver.unsubscribe();
  println!("Simulation running.");
  simulation_threads.push(thread::spawn(move || {
    for block in block_receiver {
      println!(
        "Block {} has {} transactions.",
        block.height,
        block.transactions.len()
      )
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
