use super::state::Transaction;
use accumulator::group::UnknownOrderGroup;
use accumulator::Witness;
use std::hash::Hash;

#[allow(clippy::type_complexity)]
/// Extracts the elements added and deleted in a set of `transactions`.
pub fn elems_from_transactions<G: UnknownOrderGroup, T: Clone + Hash>(
  transactions: &[Transaction<G, T>],
) -> (Vec<T>, Vec<(T, Witness<G, T>)>) {
  let mut elems_added = Vec::new();
  let mut elems_deleted = Vec::new();

  for tx in transactions {
    elems_added.extend(tx.utxos_created.iter().cloned());
    elems_deleted.extend(
      tx.utxos_spent_with_witnesses
        .iter()
        .map(|(u, wit)| (u.clone(), wit.clone())),
    );
  }

  (elems_added, elems_deleted)
}
