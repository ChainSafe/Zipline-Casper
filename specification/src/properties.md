# Protocol Properties

We instantiated Zipline for two networks. The [Ethereum spec tests](https://github.com/ethereum/consensus-spec-tests) and the Ethereum mainnet. For both of these we measured the number of instructions required to verify one epoch finality as well as the gas cost for submitting an update and for initiating a challenge.

Emulation time was measured on a 2023 Macbook M2 with 24GB of ram.

## Spec Tests

Network Params

| |  |
| -------- | -------- |
| Validators | 256 |
| Attestations | 96 |

Results

| |  |
| -------- | -------- |
| MIPS instructions | 15497874166 |
| Time to emulate | 160s |
| Input size | 38KB |
| Gas to submit | 93843 |
| Gas to challenge | 5124231 |
| Gas per dissection | 2981624 |
## Mainnet

TBD
