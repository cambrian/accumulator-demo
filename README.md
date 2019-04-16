# accumulator-demo
A proof-of-concept for stateless Bitcoin nodes, based on the
[accumulator](https://github.com/cambrian/accumulator) crate. Provided totally as-is and probably
will not be maintained, though the authors of this simulation are more than happy to answer your
questions.

## Setup
See the [accumulator](https://github.com/cambrian/accumulator/blob/master/CONTRIBUTING.md) repo for
general setup instructions.

## Usage
Just `cargo run`.

## Docs
The concept for this simulation is adapted from _Batching Techniques for Accumulators with
Applications to IOPs and Stateless Blockchains_ (Boneh, Bünz, and Fisch 2018)
[[Link]](https://eprint.iacr.org/2018/1188.pdf).

We envision three kinds of actors in a stateless Bitcoin ecosystem (some familiarity with Bitcoin
and accumulators is assumed):
- **Miners** aggregate transactions into blocks, establish consensus on each new block, and store
  the current chain state in an accumulator (a constant-size aggregation of the UTXO set). Miners
  publish updates to this accumulator value.
- **Users** issue transactions from the UTXOs in their possession. When a user presents a UTXO to be
  spent, they must also present the accumulator witness for that UTXO (proving that the UTXO is
  unspent with respect to the current accumulator value). In common terminology, users can be
  understood as light clients.
- **Bridges** manage witnesses for a number of users, offering these users a liveness guarantee so
  they don't miss accumulator updates. There are efficient procedures to update a batched set of
  witnesses, and users can query their individual witnesses on-demand.

In our simulation, each user issues a single transaction per block, and miners cut blocks every `t`
milliseconds. There are `n` users assigned to each of `m` bridge nodes, for a total of `n * m` users
in the system. Although we include `r` different miners in the simulation, one of them is always
elected leader to establish consensus.

For more details, please review our code.
