# Stage 1: Builder stage
FROM nvidia/cuda:11.8.0-devel-ubuntu20.04 AS builder

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

# # Add SSH keys to the container securely
# # Here, we expect to pass the SSH key via build secrets (from host machine)
# # It’s critical to keep the private key secret and not include it in the final image

# # Set up the SSH key (from build argument)
# ARG SSH_PRIVATE_KEY
# RUN mkdir -p /root/.ssh && \
#     echo "$SSH_PRIVATE_KEY" > /root/.ssh/id_ed25519 && \
#     chmod 600 /root/.ssh/id_ed25519

# # Avoid StrictHostKeyChecking to prevent host verification issues
# RUN touch /root/.ssh/config && echo "Host *\n\tStrictHostKeyChecking no\n" >> /root/.ssh/config

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

# Build the docker and clone private dependencies using SSH
RUN --mount=type=ssh cargo build --release

# Verify the binary exists
RUN ls -la /usr/src/tmc-gpu-accel/target/release/

# Stage 2: Final runtime image
FROM nvidia/cuda:11.8.0-base-ubuntu20.04

# Set the working directory
WORKDIR /usr/local/bin/

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/tmc-gpu-accel/target/release/tmc-gpu-accel .

# Ensure the NVIDIA libraries are properly configured
ENV NVIDIA_VISIBLE_DEVICES all
ENV NVIDIA_DRIVER_CAPABILITIES compute,utility

# Set the entrypoint to the compiled Rust binary
ENTRYPOINT ["./tmc-gpu-accel"]
