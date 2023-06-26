# Future Work

There are a number of places where the efficiency of the protocol could be improved to both reduce the cost of a challenge game and reduce the length of the trace and thus the time required to run the emulator off-chain.

## Slot based Finalization 

Currently Zipline is designed to only finalize epoch boundary blocks (EBBs). This was chosen as this is the minimal increment in which the Casper protocol can advance. In reality bridge applications will want to access individual block headers in order to prove transaction inclusion or state.

This is still possible to do with Zipline but it does require consuming code to provide SSZ proofs of ancestry from a finalized block to the block of interest. 

A small modification to Zipline would have relayers submitting all intermediate block roots since the last finalized checkpoint in a batch. A watcher would check every root and be able to challenge if any are incorrect. The verifier would just need to accept an additional input which contains the SSZ proofs linking these candidate blocks to the candidate checkpoint.

This modification would shift the ancestry proof checking from the runtime to the provable execution making it much cheaper for consuming applications.

## Out-of-MIPS Key Decompression

The beacon state stores the BLS public keys in compressed curve form. That is only the $x$ coordinate of the elliptic curve point is stored plus a bit indicating if the $y$ coordinate is positive or negative. 

Compressed keys can be decompressed by computing the $y$ coordinate which requires computing a modulo square root. This decompression operation is by far the most expensive part of the finality client verification.

This expensive operation could be eliminated by computing the y-coordinates natively (e.g. not in the emulator) and providing them as input the the pre-image oracle. A commitment to a SSZ Merkle data structure containing the y-coordinates could be added to the `ZiplineInput` container. Checking the correctness of a y-coordinate is far cheaper than computing it and only requires a single evaluation of the curve equation. 

It is expected that this optimization would significantly reduce the size of the execution trace and the verification time.

## Attestation Signature Aggregation (Super Attestations)

Currently all attestations required to prove finality must be submitted on-chain when a call to `challenge()` is made. This represents significant calldata cost to the challenger. Even though they will recover the cost by winning the challenge game it increases the size of the bond required and increases the amount of block space required to initiate a challenge this making DoS attacks cheaper to conduct.

The beacon chain takes advantage of BLS signature aggregation by aggregating committee signatures using elliptic curve addition. This works because all committee members sign the exact same message \\(H\\). Each individual signature is given by 

\\[S_i = x_i \cdot H\\]

where \\(x_i\\) is the ith validator private key. The aggregate is calculated as

\\[S = \sum S_i\\]

and the aggregate key is

\\[X = \sum X_i\\]

where \\(X_i\\) is the ith validator public key.

Verifying requires using the pairing operation to check

\\[e(S, G) == e(H, X)\\].

This check is done once per committee per epoch. For a finality check this means `n_committees * n_epochs` attestations must be submitted and double that number of pairing operations are required to check all the attestation signatures.

---

BLS also allows signature aggregation over heterogeneous messages. For unique messages \\(H_j\\) the aggregate signature is calculated in the same way as above. The signature verification becomes a check of

\\[e(S, G) == \sum_j e\left(H_j, \sum^i_{i \in signer(j)} X\right)\\].

This alteration would reduce the number of signatures required to submit in calldate from `n_committees * n_epochs` to 1. It would also reduce the number of pairing operations from `2 * n_committees * n_epochs` to `n_committees * n_epochs + 1`. This reduction in calldata cost and execution complexity would make this a valuable addition to a production implementation.
