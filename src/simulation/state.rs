use accumulator::group::UnknownOrderGroup;
use accumulator::{Accumulator, MembershipProof};
use uuid::Uuid;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Utxo {
  pub id: Uuid,
  pub user_id: Uuid,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Transaction<G: UnknownOrderGroup> {
  pub block_height: u64,
  pub utxos_created: Vec<Utxo>,
  pub utxos_spent_with_witnesses: Vec<(Utxo, Accumulator<G>)>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Block<G: UnknownOrderGroup> {
  pub height: u64,
  pub transactions: Vec<Transaction<G>>,
  pub acc_new: Accumulator<G>,
  pub proof_added: MembershipProof<G>,
  pub proof_deleted: MembershipProof<G>,
}
