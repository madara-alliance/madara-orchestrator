# Madara Orchestrator 🎭

Madara Orchestrator is a service that runs parallel to Madara, managing critical jobs for block processing, proof generation, data submission, and state transitions.

> 📝 **Note**: These instructions are verified for Ubuntu systems. While most steps remain similar for macOS, some package names and installation commands may differ.

## Table of Contents

- [Overview](#overview)
- [Architecture](#architecture)
- [Build Requirements](#build-requirements)
- [Building from Source](#building-from-source)
- [Deployment](#deployment)
  - [Local Development](#local-development)
  - [Production](#production)
- [Configuration](#configuration)
- [Testing](#testing)
- [Monitoring](#monitoring)
- [Error Handling](#error-handling)

## Overview

The Madara Orchestrator coordinates five primary jobs through a centralized queue system:

1. **SNOS (Starknet OS) Job**: Processes blocks and generates Program Independent Execution (PIE)
2. **Proving Job**: Manages proof generation using PIE
3. **Data Submission Job**: Handles state updates for availability layers (Currently supports Ethereum EIP-4844)
4. **State Transition Job**: Manages state transitions with settlement layers (Ethereum and Starknet)

## Architecture

### Job Processing Model

The orchestrator uses a queue-based architecture with three execution phases:
1. Creation: Jobs are spawned based on block availability
2. Processing: Core job logic execution
3. Verification: Result validation and confirmation

### Queue Structure

- Worker Trigger Queue
- SNOS Processing/Verification Queues
- Proving Processing/Verification Queues
- Data Submission Processing/Verification Queues
- State Update Processing/Verification Queues
- Job Failure Handling Queue

### Workflow

1. Cron jobs trigger worker tasks
2. Workers determine block-level requirements
3. Jobs are created and added to processing queues
4. Processed jobs move to verification queues
5. Verified jobs are marked complete in the database

## Build Requirements

### System Dependencies
> For macOS users, use `brew install` instead of `apt install` for these dependencies.

- Build essentials (`build-essential`)
- OpenSSL (`libssl-dev`)
- Package config (`pkg-config`)
- Python 3.9 with development files
- GMP library (`libgmp-dev`)

### Core Dependencies
- [Git](https://git-scm.com/book/en/v2/Getting-Started-Installing-Git)
- [Rust](https://www.rust-lang.org/tools/install)
- [Madara Node](https://github.com/madara-alliance/madara)
- Prover Service (ATLANTIC)

## Building from Source

1. **Install System Dependencies**
   ```bash
   # Ubuntu/Debian
   sudo apt-get update
   sudo apt install build-essential openssl pkg-config libssl-dev
   sudo apt install python3.9 python3.9-venv python3.9-distutils libgmp-dev python3.9-dev

   # For macOS
   brew install openssl pkg-config gmp python@3.9
   ```

2. **Install Rust** (Cross-platform)
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   source ~/.bashrc  # Or source ~/.zshrc for macOS
   ```

3. **Clone Repository**
   ```bash
   git clone https://github.com/madara-alliance/madara-orchestrator.git
   cd madara-orchestrator
   git submodule update --init
   ```

4. **Build SNOS**
   ```bash
   make snos
   ```

5. **Build Project**
   ```bash
   cargo build --release
   ```

## Deployment

### Local Development

#### Additional Requirements
1. **Docker** - For running local services
2. **MongoDB** - For job management
3. **Localstack** - For simulating AWS services
4. **Anvil** - For local Ethereum node
5. **Foundry** - For Ethereum development tools

#### Setup Steps

1. **Install Docker** (Cross-platform)
   Follow the official installation guides:
   - [Ubuntu Installation Guide](https://docs.docker.com/engine/install/ubuntu/#install-using-the-repository)
   - [macOS Installation Guide](https://docs.docker.com/desktop/install/mac-install/)

2. **Install Foundry** (Cross-platform)
   ```bash
   curl -L https://foundry.paradigm.xyz | bash
   foundryup
   ```

3. **Start Local Services**
   ```bash
   # Start MongoDB
   docker run -d -p 27017:27017 mongo

   # Start Localstack
   docker run -d -p 4566:4566 localstack/localstack@sha256:763947722c6c8d33d5fbf7e8d52b4bddec5be35274a0998fdc6176d733375314

   # Start Anvil in a separate terminal
   anvil --block-time 1
   ```

4. **Initialize Local Infrastructure**
   ```bash
   cargo run --release --bin orchestrator setup --aws --aws-s3 --aws-sqs --aws-sns --aws-event-bridge
   ```

5. **Run Orchestrator Locally**
   ```bash
   RUST_LOG=info cargo run --release --bin orchestrator run \
       --atlantic \
       --aws \
       --settle-on-ethereum \
       --aws-s3 \
       --aws-sqs \
       --aws-sns \
       --da-on-ethereum \
       --mongodb
   ```

### Production

#### Requirements
- AWS Account with access to:
  - S3
  - SQS
  - SNS
  - EventBridge
- MongoDB instance
- Ethereum RPC endpoint
- ATLANTIC prover service access

#### Deployment Steps

1. **Configure AWS Services**
   See [Configuration](#configuration) section for required environment variables.
   ```bash
   cargo run --release --bin orchestrator setup \
       --aws \
       --aws-s3 \
       --aws-sqs \
       --aws-sns \
       --aws-event-bridge
   ```

2. **Run Orchestrator**
   ```bash
   RUST_LOG=info cargo run --release --bin orchestrator run \
       --atlantic \
       --aws \
       --settle-on-ethereum \
       --aws-s3 \
       --aws-sqs \
       --aws-sns \
       --da-on-ethereum \
       --mongodb
   ```

## Configuration

Create a `.env` file with appropriate values for your environment:

### Production Environment
```env
# AWS Configuration
AWS_ACCESS_KEY_ID=<your-key>
AWS_SECRET_ACCESS_KEY=<your-secret>
AWS_REGION=<your-region>

# Prover Configuration
MADARA_ORCHESTRATOR_ATLANTIC_API_KEY=<api-key>
MADARA_ORCHESTRATOR_ATLANTIC_SERVICE_URL=<service-url>

# Database Configuration
MADARA_ORCHESTRATOR_MONGODB_CONNECTION_URL=<production-mongodb-url>
MADARA_ORCHESTRATOR_DATABASE_NAME=orchestrator

# RPC Configuration
MADARA_ORCHESTRATOR_ETHEREUM_SETTLEMENT_RPC_URL=<ethereum-rpc-url>
MADARA_ORCHESTRATOR_RPC_FOR_SNOS=<snos-rpc-url>
```

### Local Development
```env
# AWS Configuration (Localstack)
AWS_ACCESS_KEY_ID=test
AWS_SECRET_ACCESS_KEY=test
AWS_REGION=us-east-1

# Database Configuration
MADARA_ORCHESTRATOR_MONGODB_CONNECTION_URL=mongodb://localhost:27017
MADARA_ORCHESTRATOR_DATABASE_NAME=orchestrator

# RPC Configuration
MADARA_ORCHESTRATOR_ETHEREUM_SETTLEMENT_RPC_URL=http://localhost:8545
MADARA_ORCHESTRATOR_RPC_FOR_SNOS=<local-snos-rpc-url>
```

## Testing

### Local Environment Tests
Requires [local services](#local-development) to be running (MongoDB, Localstack, Anvil)

1. **E2E Tests**
   ```bash
   RUST_LOG=info cargo test --features testing test_orchestrator_workflow -- --nocapture
   ```

2. **Integration and Unit Tests**
   ```bash
   RUST_LOG=debug RUST_BACKTRACE=1 cargo llvm-cov nextest \
       --release \
       --features testing \
       --lcov \
       --output-path lcov.info \
       --test-threads=1 \
       --workspace \
       --exclude=e2e-tests \
       --no-fail-fast
   ```

## Monitoring

The orchestrator includes [OpenTelemetry](https://opentelemetry.io/) integration for monitoring:
- Job execution metrics
- Processing time statistics 
- RPC performance metrics

Configure monitoring using:
- `--otel-service-name`: OpenTelemetry service name
- `--otel-collector-endpoint`: OpenTelemetry collector endpoint

## Error Handling

- Failed jobs move to a dedicated failure queue (see [Queue Structure](#queue-structure))
- Automatic retry mechanism with configurable intervals
- Failed jobs tracked in database after maximum retries
- Integrated telemetry for failure monitoring (see [Monitoring](#monitoring))

For additional architectural details, refer to `./docs/orchestrator_da_sequencer_diagram.png`