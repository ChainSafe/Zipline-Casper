default:
  just --list

download-integration-tests: clean-integration-tests
    #!/usr/bin/env sh
    TESTS_TAG=v1.1.10
    REPO_NAME=consensus-spec-tests
    CONFIGS="general minimal mainnet"
    mkdir ${REPO_NAME}
    for config in ${CONFIGS}
    do
        wget https://github.com/ethereum/${REPO_NAME}/releases/download/${TESTS_TAG}/${config}.tar.gz
        tar -xzf ${config}.tar.gz -C ${REPO_NAME}
    done
    rm -f *tar.gz
clean-integration-tests:
    rm -rf consensus-spec-testss

cache-zipline-tests: # warning this is slow. Takes a few minutes
    cargo test --release -p zipline-finality-client -- --ignored --nocapture --skip unicorn_mainnet --skip cache_demo_files --skip native_mainnet

test:
    cargo test --release
slow-tests:
    cargo test -p zipline-finality-client native_mainnet unicorn_mainnet -- --ignored
fmt:
    cargo fmt --all
lint: fmt
    cargo clippy --all-targets --all-features
build:
    cargo build --all-targets --all-features
run-ci: lint build test
