use accumulator::group::UnknownOrderGroup;
use accumulator::{Accumulator, MembershipProof, Witness};
use std::hash::Hash;
use uuid::Uuid;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
/// A UTXO, defined by a UUID and the user who owns it. Note that our UTXOs do not have an
/// associated value, since that is irrelevant to our simulation.
pub struct Utxo {
  pub id: Uuid,
  pub user_id: usize,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
/// A transaction, defined by UTXOs created and UTXOs spent. UTXOs being spent must come with a
/// witness to prove that they are currently unspent.
pub struct Transaction<G: UnknownOrderGroup, T: Hash> {
  pub utxos_created: Vec<T>,
  pub utxos_spent_with_witnesses: Vec<(T, Witness<G, T>)>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
/// A block, which gets cut by a miner. Blocks contain transactions, a new accumulator value, and
/// proofs of correctness for the accumulator update.
pub struct Block<G: UnknownOrderGroup, T: Hash> {
  pub height: u64,
  pub transactions: Vec<Transaction<G, T>>,
  pub acc_new: Accumulator<G, T>,
  pub proof_added: MembershipProof<G, T>,
  pub proof_deleted: MembershipProof<G, T>,
}
