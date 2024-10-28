FROM rust:1.81 AS builder

# Set the working directory in the container
WORKDIR /usr/src/madara-orchestrator

# Install system dependencies
RUN apt update && apt install -y  \
    libgmp3-dev \
    software-properties-common \
    wget \
    bash \
    nodejs \
    npm

# Install Python 3.9
RUN wget https://www.python.org/ftp/python/3.9.16/Python-3.9.16.tgz \
    && tar xzf Python-3.9.16.tgz \
    && cd Python-3.9.16 \
    && ./configure --enable-optimizations \
    && make altinstall \
    && cd .. \
    && rm -rf Python-3.9.16 Python-3.9.16.tgz

# Install pip
RUN wget https://bootstrap.pypa.io/get-pip.py \
    && python3.9 get-pip.py \
    && rm get-pip.py

# Copy the current directory contents into the container
COPY . .


# Check rust version (this also installs version from rust-toolchain file)
RUN rustup show

# #############################################################
# TODO : remove this step after snos build is sorted
# Build cairo lang
RUN cargo fetch
RUN make snos
# #############################################################

WORKDIR /usr/src/madara-orchestrator

# Build the project
RUN cargo build --release

# Install Node.js dependencies for migrations
RUN npm install


FROM debian:bookworm

# Install runtime dependencies
RUN apt-get -y update && \
    apt-get install -y openssl ca-certificates nodejs npm &&\
    apt-get autoremove -y; \
    apt-get clean; \
    rm -rf /var/lib/apt/lists/*

# Set the working directory
WORKDIR /usr/local/bin

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/madara-orchestrator/target/release/orchestrator .

# Copy Node.js files and dependencies
COPY --from=builder /usr/src/madara-orchestrator/node_modules ./node_modules
COPY --from=builder /usr/src/madara-orchestrator/package.json .
COPY --from=builder /usr/src/madara-orchestrator/migrate-mongo-config.js .
COPY --from=builder /usr/src/madara-orchestrator/migrations ./migrations

# To be fixed by this https://github.com/keep-starknet-strange/snos/issues/404
RUN mkdir -p /usr/local/cargo/git/checkouts/snos-59fe8329bb16fe65/662d170/crates/starknet-os/kzg
COPY ./crates/da-clients/ethereum/trusted_setup.txt /usr/local/cargo/git/checkouts/snos-59fe8329bb16fe65/662d170/crates/starknet-os/kzg/trusted_setup.txt
COPY ./crates/da-clients/ethereum/trusted_setup.txt /usr/src/madara-orchestrator/crates/settlement-clients/ethereum/src/trusted_setup.txt

# Create a startup script
RUN echo '#!/bin/bash\n\
    npm run migrate up\n\
    ./orchestrator' > start.sh && chmod +x start.sh

# Set the entrypoint to the startup script
ENTRYPOINT ["./start.sh"]