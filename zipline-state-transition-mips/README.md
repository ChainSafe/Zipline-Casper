# Zipline State Transition MIPS🦀💣💥

Bootstraped with [Rust-Cannon-Template](https://github.com/willemolding/rust-cannon-template)

## Building with Docker (Recommended)

Docker can be used for cross-compiling MIPS on any host. First build the docker image by running:

```shell
make docker_image
# or for Apple Silicon users
make docker_image_apple_silicon
```

After this a Cannon ready MIPS binary can be build with:
```shell
make build
```

This will write an `out.bin` file to the build directory.

## Building Locally (Tested Ubuntu 22.04 only)

The build script `build.sh` will cross-compile to the MIPS target and then post-process the resulting elf file into a binary that can be used in the MIPS emulator and proven on-chain.

This has only been tested in Ubuntu 22.04 so milage may vary

### Dependencies

```shell
sudo apt install \
    build-essential \
    g++-mips-linux-gnu \
    libc6-dev-mips-cross \
    llvm \
    clang \
    python3 python3.8-venv python3-pip 
```
### Building

Build a binary compatible with Ethereum mainnet with

```shell
./build.sh
```

This will write the Cannon compatible binary to `build/mainnet_out.bin`

You can also build a binary for verifying with the Ethereum spec tests with

```shell
SPEC=spec_test ./build.sh
```

which will output to `build/spec_test_out.bin`

---

Alternatively if you want to experiment in the build environment you can load up an interactive shell with
	
```shell
make docker_image
docker run -it --rm -v $(pwd):/code zipline-state-transition-mips/builder bash
```
(replace with your project name as required)

and from there you can run 

```shell
./build.sh
```
to produce the output
