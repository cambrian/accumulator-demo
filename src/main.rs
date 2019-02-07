//! Simulation runner.
//! TODO: set up communication channels and start threads for components

mod simulation;
use accumulator::group::Rsa2048;
use multiqueue::{broadcast_queue, BroadcastReceiver, BroadcastSender};
use simulation::{Bridge, Miner, User};
use std::collections::HashMap;
use std::thread;
use uuid::Uuid;

const NUM_MINERS: usize = 5;
const NUM_BRIDGES: usize = 2;
const NUM_USERS_PER_BRIDGE: usize = 25;
const BLOCK_INTERVAL_SECONDS: u16 = 30;

fn new_queue<T: Clone>() -> (BroadcastSender<T>, BroadcastReceiver<T>) {
  broadcast_queue(256)
}

pub fn main() {
  let mut miners = Vec::with_capacity(NUM_MINERS);
  let mut bridges = Vec::with_capacity(NUM_BRIDGES);
  let mut users = Vec::with_capacity(NUM_BRIDGES * NUM_USERS_PER_BRIDGE);

  let (block_sender, block_receiver) = new_queue();
  let (tx_sender, tx_receiver) = new_queue();

  for i in 0..NUM_MINERS {
    let miner = Miner::<Rsa2048>::setup(
      i == 0,
      BLOCK_INTERVAL_SECONDS,
      block_sender.clone(),
      block_receiver.clone(),
      tx_receiver.clone(),
    );
    miners.push(miner);
  }

  for _ in 0..NUM_BRIDGES {
    let (witness_request_sender, witness_request_receiver) = new_queue();
    let mut witness_response_senders = HashMap::new();
    for _ in 0..NUM_USERS_PER_BRIDGE {
      let (witness_response_sender, witness_response_receiver) = new_queue();
      let user_id = Uuid::new_v4();
      witness_response_senders.insert(user_id, witness_response_sender);
      let user = User::<Rsa2048>::setup(
        user_id,
        witness_request_sender.clone(),
        witness_response_receiver,
        tx_sender.clone(),
      );
      users.push(user);
    }
    let bridge = Bridge::<Rsa2048>::setup(
      block_receiver.clone(),
      witness_request_receiver,
      witness_response_senders,
    );
    bridges.push(bridge);
  }

  println!("Simulation initialized.");
  let mut simulation_threads = Vec::new();
  for mut miner in miners {
    simulation_threads.push(thread::spawn(move || miner.run()));
  }
  for mut bridge in bridges {
    simulation_threads.push(thread::spawn(move || bridge.run()));
  }
  for mut user in users {
    simulation_threads.push(thread::spawn(move || user.run()));
  }
  println!("Simulation running!");
  for thread in simulation_threads {
    thread.join().unwrap();
  }
  println!("Simulation exiting!");
}
