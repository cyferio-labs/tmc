# Stage 1: Builder stage
FROM nvidia/cuda:11.8.0-devel-ubuntu20.04 AS builder

# Install necessary dependencies
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
    cmake

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
WORKDIR /usr/src/myapp

# Copy the source code
COPY . .

# Clone private dependencies using SSH
RUN --mount=type=ssh cargo build --release

# Stage 2: Final runtime image
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
