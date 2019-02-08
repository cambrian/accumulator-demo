//! Simulation runner.
//! TODO: Add configurability/statistics.
mod simulation;
use accumulator::group::{Rsa2048, UnknownOrderGroup};
use accumulator::hash::hash_to_prime;
use accumulator::Accumulator;
use multiqueue::{broadcast_queue, BroadcastReceiver, BroadcastSender};
use simulation::state::Utxo;
use simulation::{Bridge, Miner, User};
use std::collections::HashMap;
use std::thread;
use uuid::Uuid;

// TODO: Put in separate config file?
const BLOCK_TIME_IN_SECONDS: u64 = 30;

const NUM_MINERS: usize = 5;
const NUM_BRIDGES: usize = 2;
const NUM_USERS: usize = 50;

// NOTE: Ensure that sum of USERS_ASSIGNED_TO_BRIDGE is NUM_USERS.
const USERS_ASSIGNED_TO_BRIDGE: [usize; NUM_BRIDGES] = [25; 2];
const TX_ISSUANCE_FREQS_IN_HZ: [u64; NUM_USERS] = [10; NUM_USERS];

fn new_queue<T: Clone>() -> (BroadcastSender<T>, BroadcastReceiver<T>) {
  broadcast_queue(256)
}

pub fn run_simulation<G: UnknownOrderGroup>() {
  println!("Simulation starting.");
  let mut simulation_threads = Vec::new();
  let (block_sender, block_receiver) = new_queue();
  let (tx_sender, tx_receiver) = new_queue();

  let mut init_acc = Accumulator::<G>::new();
  let mut rand_utxos = Vec::new();
  for _ in 0..(NUM_BRIDGES * NUM_USERS) {
    let rand_utxo = Utxo { id: Uuid::new_v4() };
    init_acc = init_acc.add(&[hash_to_prime(&rand_utxo)]).0;
    rand_utxos.push(rand_utxo);
  }

  for miner_idx in 0..NUM_MINERS {
    // These clones cannot go inside the thread closure, since the variable being cloned would get
    // swallowed by the move (see below as well).
    let init_acc = init_acc.clone();
    let block_sender = block_sender.clone();
    let block_receiver = block_receiver.add_stream();
    let tx_receiver = tx_receiver.add_stream();
    simulation_threads.push(thread::spawn(move || {
      Miner::<G>::launch(
        miner_idx == 0, // elect first miner as leader
        init_acc,
        BLOCK_TIME_IN_SECONDS,
        block_sender,
        block_receiver,
        tx_receiver,
      )
    }));
  }

  let mut user_idx = 0;
  #[allow(clippy::needless_range_loop)] // stfu clippy
  for bridge_idx in 0..NUM_BRIDGES {
    let (witness_request_sender, witness_request_receiver) = new_queue();
    let mut witness_response_senders = HashMap::new();

    for _ in 0..USERS_ASSIGNED_TO_BRIDGE[bridge_idx] {
      let (witness_response_sender, witness_response_receiver) = new_queue();
      let user_id = Uuid::new_v4();
      witness_response_senders.insert(user_id, witness_response_sender);

      let init_utxo = rand_utxos[user_idx].clone();
      let witness_request_sender = witness_request_sender.clone();
      let tx_sender = tx_sender.clone();
      simulation_threads.push(thread::spawn(move || {
        User::launch(
          user_id,
          init_utxo,
          TX_ISSUANCE_FREQS_IN_HZ[user_idx],
          witness_request_sender,
          witness_response_receiver,
          tx_sender,
        );
      }));
      user_idx += 1;
    }

    let block_receiver = block_receiver.add_stream();
    let init_acc = init_acc.clone();
    simulation_threads.push(thread::spawn(move || {
      Bridge::<G>::launch(
        init_acc,
        block_receiver,
        witness_request_receiver,
        witness_response_senders,
      );
    }));
  }

  println!("Simulation running.");
  for thread in simulation_threads {
    thread.join().unwrap();
  }
  println!("Simulation exiting.");
}

pub fn main() {
  run_simulation::<Rsa2048>();
}
