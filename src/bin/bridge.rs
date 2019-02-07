extern crate accumulator;
use accumulator::group::Rsa2048;
use accumulator::Accumulator;
use accumulator_demo::simulation::Bridge;

pub fn main() {
  let _bridge = Bridge::<Rsa2048>::setup(Accumulator::new(), 0);
  println!("Hello!")
}
