![](./specification/src/graphics/banner.png)

[![Apache V2 License](https://img.shields.io/github/license/ChainSafe/Zipline-Casper.svg?style=for-the-badge)](https://github.com/ChainSafe/Zipline-Casper/blob/master/LICENSE)
[![GitHub Workflow Status](https://img.shields.io/github/actions/workflow/status/ChainSafe/Zipline-Casper/zipline.yml?style=for-the-badge)](https://github.com/ChainSafe/Zipline-Casper/actions)
[![Specification](https://img.shields.io/badge/doc-book-green?style=for-the-badge)](https://chainsafe.github.io/Zipline-Casper/)

Zipline is a permissionless block header oracle from Gasper chains (Ethereum/Gnosis beacon chain) to EVM chains! It uses fault proofs to ensure that any relayed block roots that have not been finalized by the origin chain can be proven fraudulent.

This repo contains a proof-of-concept implementation of the main components that make up the protocol. Primarily the [contracts](./contracts) that run on the destination chain and the [provable execution](./zipline-state-transition-mips/) that runs in an emulator off-chain (but can be proven on-chain) to verify chain finality.

It also contains [a book](https://chainsafe.github.io/Zipline-Casper/) describing the design of the protocol in detail. 

## Repo Overview

| Component | Description | Doc |
| -------- | -------- | -------- |
| Contracts     | The Destination chain contracts. These implement the fraud proving game and store the trusted origin chain checkpoints   | [![documentation](https://img.shields.io/badge/readme-blue)](./contracts)  |
| Demo | A demo that can be run locally showing the different components interact to produce the protocol | [![documentation](https://img.shields.io/badge/readme-blue)](./demo)
| Finality Client | A Rust implementation of a Casper finality client | [![documentation](https://img.shields.io/badge/readme-blue)](./finality-client) |
Preprocessor | A typescript CLI and daemon that poll a Gasper chain to produce proofs for a finality client | [![documentation](https://img.shields.io/badge/readme-blue)](./preprocessor) |
Hashi Adapter | An example of integrating Zipline with the [Hashi](https://github.com/gnosis/hashi) EVM block oracle aggregator | [![documentation](https://img.shields.io/badge/readme-blue)](./hashi-adapter) | 
Zipline State Transition (MIPS) | A Rust implementation of the Zipline state transition function that builds to a [Cannon](https://github.com/ethereum-optimism/cannon) compatible MIPS binary. This wraps the finality client in a form that can be run in the emulator and proven on-chain | [![documentation](https://img.shields.io/badge/readme-blue)](./zipline-state-transition-mips) | 
Emulator | A Rust implementation of a MIPS emulator that is compatible with Cannon | [![documentation](https://img.shields.io/badge/readme-blue)](./emulator) | 
Specification | An MDBook describing the protocol in detail. Also hosted at [https://chainsafe.github.io/Zipline-Casper](https://chainsafe.github.io/Zipline-Casper) | [![documentation](https://img.shields.io/badge/readme-blue)](https://chainsafe.github.io/Zipline-Casper) |

## Demo

Want to see it run for yourself? The easiest way is to run the [demo script](./demo). This will create a local testnet destination chain using [anvil](https://book.getfoundry.sh/reference/anvil/) and simulate two actors interacting with the protocol and show how one can successfully prove fraud in the case a non-finalized origin chain block is submitted.

## Prerequisites

This repo uses [just](https://github.com/casey/just) for running scripts. You can install it with cargo.

## Authors

- Willem Olding
- Eric Tu
- Cayman Nava

⚠️ ❌ ❗️❗️NOT PRODUCTION READY❗️❗️❌ ⚠️
