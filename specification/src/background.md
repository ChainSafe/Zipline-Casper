# Background

## Casper Finality Gadget

Casper is the finality protocol used as part of the beacon chain protocol. It provides an economic guarantee that once a checkpoint has been finalized it cannot be removed from the canonical chain without slashing 1/3 of all stake is slashed. This 1/3 of stake is the economic security of the finality.

Casper operates on checkpoints which are pairs of epoch block boundaries and epoch numbers. Finalizing a checkpoint serves to also finalize all ancestors of the checkpoint. Validators in Casper vote by signing messages which contain two checkpoints - a source and a target. By signing a source-target pair a validator attests that both are part of the canonical chain to the best of their knowledge.

When 2/3 of validators have attested to the same source-target pair it is known as a *supermajority link*. A supermajority link with a justified source results in the target also being justified. A supermajority link with a justified source, a justified target, and all intermediate checkpoints also being justified, results in the source being *finalized*. 

A justified checkpoint cannot be un-justified without a slashing event however it is possible for their to be multiple justified checkpoints at the same height. This is unlike a finalized checkpoints for which there cannot be another conflicting checkpoint under the economic security assumptions. This property is important in the design of the Zipline protocol.

For further reading see the [Gasper paper](https://arxiv.org/pdf/2003.03052.pdf).

## Optimism Cannon

Cannon is a fault proving framework developed as part of the Optimism stack. It provides a way to write and compile programs such that the result of their execution can be proven on-chain using interactive fault proofs.

Offchain the programs are executed in an emulator. This has access to all registers and memory for the CPU at any point during program execution. The sequence of these memory snapshots for the execution of a program is called a trace. For any initial snapshot there is exactly one correct trace.

### Pre-image Oracle

Cannon has a feature that makes it possible to read certain pieces data from the outside world during execution called the pre-image oracle. The program can write a hash into a special location in memory and expect the pre-image of that hash to appear in another memory range. The program will only continue execution if the host correctly inserts this pre-image into memory.

Obviously this is impossible in the general case as hash functions should be irreversible. It can only be guaranteed to work for data that is known to be available in both hashed and un-hashed form. A good example of this would be blocks in the host chain. Both the block hashes and block data are available in the chain history and can reasonable be assumed to be available.

During the final stage of fault-proof execution if the executed instruction is reading from the pre-image oracle output memory range then the contract must be provided with the pre-image data in order to hash it and check its correctness.
