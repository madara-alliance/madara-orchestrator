# Use latest Ubuntu
FROM ubuntu:latest

# Install necessary runtime dependencies
RUN apt-get update && \
    apt-get install -y \
    ca-certificates \
    && \
    apt-get clean && \
    rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /usr/local/bin

# Copy the pre-built binary
COPY target/release/orchestrator .

# Copy Node.js files and dependencies
COPY node_modules ./node_modules
COPY package.json .
COPY migrate-mongo-config.js .
COPY migrations ./migrations

# Create required directories and copy setup files
RUN mkdir -p /home/ubuntu/.cargo/git/checkouts/snos-59fe8329bb16fe65/2c47281/crates/starknet-os/kzg
RUN mkdir -p /usr/src/madara-orchestrator/crates/settlement-clients/ethereum/src

# Copy trusted setup files to required locations
COPY ./crates/da-clients/ethereum/trusted_setup.txt /home/ubuntu/.cargo/git/checkouts/snos-59fe8329bb16fe65/2c47281/crates/starknet-os/kzg/trusted_setup.txt
COPY ./crates/da-clients/ethereum/trusted_setup.txt /home/ubuntu/madara-orchestrator/crates/settlement-clients/ethereum/src/trusted_setup.txt

# Make sure the binary is executable
RUN chmod +x orchestrator

# Set the entrypoint to the binary
ENTRYPOINT ["./orchestrator"]
# Allow passing arguments
CMD []
