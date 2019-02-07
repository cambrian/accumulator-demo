/// Simulation runner.
extern crate accumulator;
use accumulator::group::Rsa2048;
use accumulator_demo::simulation::miner::Miner;
use accumulator_demo::simulation::bridge::Bridge;
use accumulator_demo::simulation::user::User;
use std::collections::HashSet;
use accumulator::Accumulator;

pub fn main() {
  let _miner = Miner::<Rsa2048>::setup(true, Accumulator::new(), 0);
  let _bridge = Bridge::<Rsa2048>::setup(Accumulator::new(), 0);
  let _user = User::setup(HashSet::new());
  println!("Hello!")
}
