# Use the official CUDA image as the base
FROM nvidia/cuda:11.8.0-devel-ubuntu20.04 AS builder

# Install necessary dependencies
RUN apt-get update && apt-get install -y --no-install-recommends \
    build-essential \
    clang \
    libclang-dev \
    curl \
    ca-certificates \
    git

# Install Rust toolchain
RUN curl https://sh.rustup.rs -sSf | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

# Set the working directory inside the container
WORKDIR /usr/src/myapp

# Copy the source code
COPY . .

# Install necessary Rust dependencies and compile the application
RUN cargo build --release

# Create a smaller runtime image with the compiled binary
FROM nvidia/cuda:11.8.0-base-ubuntu20.04

# Set the working directory
WORKDIR /usr/local/bin/

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/myapp/target/release/myapp .

# Ensure the NVIDIA libraries are properly configured
ENV NVIDIA_VISIBLE_DEVICES all
ENV NVIDIA_DRIVER_CAPABILITIES compute,utility

# Set the entrypoint to the compiled Rust binary
ENTRYPOINT ["./myapp"]
