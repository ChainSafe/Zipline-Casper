
docker_image:
	docker build -f ./Dockerfile .. -t zipline-state-transition-mips/builder 

docker_image_apple_silicon:
	docker buildx build --platform linux/amd64 -f ./Dockerfile .. -t zipline-state-transition-mips/builder 

.PHONY: build

build:
	docker run --rm -e SPEC='mainnet' -v $(shell pwd)/..:/code/ zipline-state-transition-mips/builder
build_spectest_spec:
	docker run --rm -e SPEC='spec_test' -v $(shell pwd)/..:/code/ zipline-state-transition-mips/builder
build_minimal_spec:
	docker run --rm -e SPEC='minimal' -v $(shell pwd)/..:/code/ zipline-state-transition-mips/builder

interactive:
	docker run --rm -it -v $(shell pwd)/..:/code/  --entrypoint=/bin/bash zipline-state-transition-mips/builder

ci_install_deps:
	apt-get update -y
	apt-get install --assume-yes --no-install-recommends \
	build-essential \
    curl \
    g++-mips-linux-gnu \
    libc6-dev-mips-cross \
    make \
    cmake \
    git 
    # python3 python3.9-venv python3-pip

