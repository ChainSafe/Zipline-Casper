# Known Issues and Attacks

The implementation provided is a proof-of-concept only and has a number of known issues that make it unsuitable for production deployment at this time.

## Delayed Chain Finality

The stateless finality client currently operates on the assumption that the origin chain is finalizing every epoch according to the 1-finality rule. While this is the case the vast majority of the time there are instances where chain finality can be delayed. This was recently seen in [Ethereum mainnet due to a differing client implementations](https://ethdaily.substack.com/p/ethereum-beacon-chain-finality-incident). Zipline must be able to deal with these cases in order to continue to follow the chain.

There is no significant difficulty in implementing this feature but it must be done with care to prevent long range fork style attacks using the state patches.

## Pending Checkpoint Dependencies

Currently the challenge period is longer than one epoch for the ethereum and gnosis beacon chains. This means that, similar to the Optimism rollup, there will at any point in time be a number of checkpoints that are pending that build on each other. As the finality window moves forward in time these will become finalized. The number of pending checkpoints waiting for finality should be a function of the challenge window and the epoch length.

A successful challenge on a historical checkpoint should invalidate all successive checkpoints that depend on it. This functionality is not currently implemented.

## Payload Manipulation

The Zipline challenge method will accept an arbitrary blob of data, check the first 80 bytes for consistency with the challenge and then hash it to be loaded as input to the provable execution. Some care has been taken to ensure that any input data will result in termination of the MIPS verifier however it may be possible that some input is able to result in a non-terminating program. A program that doesn't terminate cannot be proven fraudulent and this a challenger that submits this input will always win regardless of the honest of the other participant.

An audit should take special care to ensure that the program will terminate within some number of instructions regardless of the input.

## Handling of chain upgrades

The current implementation of the finality client only supports a single beacon chain fork version (e.g. Altair, Bellatrix) at a time. It does not support moving between fork versions and of course it cannot know what future fork versions will be. Any production deployment of Zipline would need to be upgradable in order to follow upgrades on its origin chain. This property is shared by all light-client based bridges and is a good argument for bridges to be secured by multiple strategies.
