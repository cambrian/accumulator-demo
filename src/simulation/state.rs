use accumulator::group::UnknownOrderGroup;
use accumulator::{Accumulator, MembershipProof};
use uuid::Uuid;

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Utxo {
  id: Uuid,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
// TODO: Maybe don't use pub(super) everywhere.
pub struct Transaction<G: UnknownOrderGroup> {
  pub(super) utxos_added: Vec<Utxo>,
  pub(super) utxos_deleted: Vec<(Utxo, Accumulator<G>)>,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub struct Block<G: UnknownOrderGroup> {
  pub(super) height: u64,
  pub(super) transactions: Vec<Transaction<G>>,
  pub(super) new_acc: Accumulator<G>,
  pub(super) proof_added: MembershipProof<G>,
  pub(super) proof_deleted: MembershipProof<G>,
}
