extern crate accumulator;
use accumulator::group::Rsa2048;
use accumulator::Accumulator;
use accumulator_demo::simulation::Miner;

pub fn main() {
  let _miner = Miner::<Rsa2048>::setup(true, Accumulator::new(), 0);
  println!("Hello!")
}
