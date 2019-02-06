/// User executable for simulation.
extern crate accumulator_demo;
use accumulator_demo::simulation::user::User;
use std::collections::HashSet;

pub fn main() {
  let _user = User::setup(HashSet::new());
  println!("Hello!")
}
