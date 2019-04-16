use accumulator::group::UnknownOrderGroup;
use accumulator::{Accumulator, MembershipProof, Witness};
use std::hash::Hash;
use uuid::Uuid;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Utxo {
  pub id: Uuid,
  pub user_id: usize,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Transaction<G: UnknownOrderGroup, T: Hash> {
  pub utxos_created: Vec<T>,
  pub utxos_spent_with_witnesses: Vec<(T, Witness<G, T>)>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Block<G: UnknownOrderGroup, T: Hash> {
  pub height: u64,
  pub transactions: Vec<Transaction<G, T>>,
  pub acc_new: Accumulator<G, T>,
  pub proof_added: MembershipProof<G, T>,
  pub proof_deleted: MembershipProof<G, T>,
}
