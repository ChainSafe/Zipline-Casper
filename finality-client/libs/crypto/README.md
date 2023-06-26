# Hashing

This crate allows us to abstract over MIPS compatible hashing algorithms and potentially swap them out for different/faster implementations with minimal code changes.

The code from this crate is a simplified version of the [eth2_hashing crate](https://github.com/sigp/lighthouse/tree/319cc61afeb1dbf3692e280dfa18e7b455542b16/crypto/eth2_hashing) but only exposes a single implementation and doesn't dynamically chose an implementation based on CPU.

It has also been patched to work in a `restricted_std` setting
