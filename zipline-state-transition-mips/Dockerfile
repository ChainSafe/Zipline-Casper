FROM ubuntu:22.04

ENV SHELL=/bin/bash
ENV DEBIAN_FRONTEND noninteractive
ENV RUST_VERSION nightly-2023-05-03

RUN apt-get update && apt-get install --assume-yes --no-install-recommends \
    ca-certificates \
    build-essential \
    curl \
    g++-mips-linux-gnu \
    libc6-dev-mips-cross \
    llvm \
    clang \
    make \
    cmake \
    git \
    python3 python3-venv python3-pip 


RUN pip3 install wheel

ENV CC_mips_unknown_none=mips-linux-gnu-gcc \
    CXX_mips_unknown_none=mips-linux-gnu-g++ \
    CARGO_TARGET_MIPS_UNKNOWN_NONE_LINKER=mips-linux-gnu-gcc

#
# Install Rustup and Rust
#
RUN curl https://sh.rustup.rs -sSf | bash -s -- -y --default-toolchain ${RUST_VERSION} --component rust-src
ENV PATH="/root/.cargo/bin:${PATH}"

# Used for build caching
RUN cargo install cargo-chef --locked

WORKDIR /code

# note this is using the directory root as its context
COPY . .

WORKDIR /code/zipline-state-transition-mips

RUN git config --global --add safe.directory '*'

# Generate recipe
RUN cargo +${RUST_VERSION} chef prepare --recipe-path recipe.json

# # Download and build depdencies
RUN cargo +${RUST_VERSION} chef cook --release --recipe-path recipe.json

CMD ["/bin/bash", "build.sh"]
