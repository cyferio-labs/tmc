# # Stage 1: Builder stage
# FROM nvidia/cuda:11.8.0-devel-ubuntu20.04 AS builder
FROM nvidia/cuda:11.8.0-devel-ubuntu20.04

ENV TZ=Asia/Taipei \
    DEBIAN_FRONTEND=noninteractive

# Install necessary dependencies
# CMAKE 3.24.0 is required for tfhe-cuda-backend v0.3.0
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    clang \
    libclang-dev \
    curl \
    ca-certificates \
    git \
    openssh-client \
    pkg-config \
    libssl-dev \
    wget \
    && wget https://github.com/Kitware/CMake/releases/download/v3.24.0/cmake-3.24.0-linux-x86_64.sh \
    && chmod +x cmake-3.24.0-linux-x86_64.sh \
    && ./cmake-3.24.0-linux-x86_64.sh --skip-license --prefix=/usr/local

# Add the Rust toolchain
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Set the working directory inside the container
WORKDIR /usr/src/tmc-gpu-accel

# Copy the source code
COPY . .

# Install risc0 toolchain
RUN cargo install cargo-binstall

# Install cargo-risczero by piping 'yes' to the command
RUN echo yes | cargo binstall cargo-risczero
RUN cargo risczero install

# Export necessary environment variables to temporarily skip zk prover build
ENV SKIP_GUEST_BUILD=1 \
    SOV_PROVER_MODE=skip

# Build node binary
RUN --mount=type=ssh cargo build --release --bin node

# Create FHE keys
RUN --mount=type=ssh cargo run --release --bin fhe-keygen

# Generate request scripts for FHE module
RUN --mount=type=ssh cargo run --release --bin request-scripts-gen

# CD into the crates/rollup directory
WORKDIR /usr/src/tmc-gpu-accel/crates/rollup

# Import keys for rollup and building wallet-cli
RUN --mount=type=ssh make import-keys

# Set the working directory of container back to project root
WORKDIR /usr/src/tmc-gpu-accel
