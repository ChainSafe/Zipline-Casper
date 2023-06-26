# Contributions

The primary innovation of Zipline is the design of a [stateless finality client](finality_client.md) for Casper. This allows client with minimal storage capabilities to follow the finalized chain by receiving only a single update message per epoch. Provable execution of this stateless client is what allows the destination chain to prove block finality.

There were also two additional findings that make implementing the protocol feasible for very large validator sets such as those on the Ethereum and Gnosis mainnets:

## 1. Strategies to reduce calldata requirements

In a typical challenge game (such as that used for an optimistic rollup bridge) a participant submits an assertion, for example the value of a new state root after some transactions have been applied. To verify this statement observers must have access to all inputs (e.g. transaction data) and to the verification program itself. To ensure these inputs are available they are typically included in calldata on the host chain.

For a Casper header oracle this is problematic because verifying finality requires a lot of data (attestations from every validator) and must be done quite frequently (once per epoch).

We solved this problem by noting two properties of a Casper finalized chain under the assumptions that the chain is live and a (1/3)-slashing event has not occurred:

1. The state for any recently finalized checkpoint can be reconstructed from public data
2. There is only a single correct finalized successor checkpoint to any finalized checkpoint

The first point allows the protocol to assume that the state for any recently finalized checkpoint is made available by the origin chain itself. Therefore the trusted checkpoint state (which is one of the main data structures required for verification) does not need to be included in calldata. This is fortunate as for any chain with a large number of validators the size of this structure is over 1GB.

The second point allows us to invert the challenge game. Rather than proving the invalidity of the execution of a finality check we can prove the validity of another update at the same height. Irrespective of the accompanying data (e.g. attestations) there is only a single validate candidate successor checkpoint to a given trusted checkpoint. A result of this is that the accompanying data need not be submitted with each update, it only needs to be included by the challenger if they decide to open a challenge. 

Both of these insights reduce the happy path operating costs to only that of receiving and storing a single checkpoint plus some additional metadata. This is far less than the original zipline light-client design and less than validity-proof based design.

## 2. Combining SSZ Merklization with a pre-image oracle for efficient structured data retrieval

[Cannon](https://github.com/ethereum-optimism/cannon), the provable execution environment used by Zipline, uses a method called the pre-image oracle for retrieval of arbitrary data by its hash. By writing a hash value into a special slot in memory the pre-image of this hash will appear in another memory range. The emulator itself (both the off-chain and on-chain variants) will effectively pause execution until the host can provide the correct pre-image. From the perspective of the code running in the emulator it has an oracle that will always correctly invert hashes.

Doing so has an associated cost when running on-chain. If the instruction executed by the on-chain arbitrator in the final stage of the challenge game reads from the pre-image oracle memory range it will require a challenge game participant to submit the correct pre-image on-chain before it can conclude. The pre-image must be submitted and hashed within the execution of the destination chain.

Our insight was that it is possible to efficiently read chunks from SSZ Merklized data structures using this pre-image oracle. Given the root hash and an index to the chunk of interest the intermediate nodes in the tree can be read one at a time from the root down. Each intermediate node contains two hashes and the index can be used to determine which side to traverse. 

Loading data this way is both memory efficient and ensures the largest size that a challenge game participant would need to submit on-chain is 2 hashes (64 bytes). This strategy is used to read chunks of data from the very large beacon state object.

