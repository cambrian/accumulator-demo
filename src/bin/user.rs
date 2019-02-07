use accumulator_demo::simulation::User;
use std::collections::HashSet;

pub fn main() {
  let _user = User::setup(HashSet::new());
  println!("Hello!")
}
