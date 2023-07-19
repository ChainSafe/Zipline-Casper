#!/usr/bin/env bash
set -e

ELF_NAME=zipline-state-transition-mips

SPEC="${SPEC:=mainnet}"

mkdir -p build

CC_mips_unknown_none=mips-linux-gnu-gcc \
CXX_mips_unknown_none=mips-linux-gnu-g++ \
CARGO_TARGET_MIPS_UNKNOWN_NONE_LINKER=mips-linux-gnu-gcc \
RUSTFLAGS="-Clink-arg=-e_start" \
CARGO_TARGET_DIR=${SPEC}_target \
    cargo +nightly-2023-05-03 build --verbose --release --target=mips-unknown-none.json  -Zbuild-std --no-default-features --features="$SPEC"

python3 -m venv venv

source venv/bin/activate
pip3 install -r requirements.txt
# builds to workspace root
./elf2bin.py ./${SPEC}_target/mips-unknown-none/release/$ELF_NAME ./build/${SPEC}_out.bin
deactivate
