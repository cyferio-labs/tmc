# TMC (Trustless Modular Calculator)

> [!IMPORTANT]  
> To enable FHE modules, please visit [`fhe-module`](https://github.com/cyferio-labs/tmc/tree/fhe-modules) branch

## Overview

TMC is a modular co-processor and rollup stack enabling verifiable Fully Homomorphic Encryption (FHE). It unlocks privacy-preserving, massively parallel execution of computations for both Web2 and Web3 applications.

By leveraging FHE, advanced modular rollup designs, and parallelism in computational proofs within a trustless computing layer, TMC enables secure, near real-time computations on both public and private on-chain states while preserving composability and interoperability.

## Key Features

- **Modular Architecture**: Highly adaptable zk-rollup framework integrating state-of-the-art privacy-preserving solutions like FHE and Zero-Knowledge Proofs (ZKPs).
  
- **Module System Interface**:
  - Supports both stateless and stateful modules, enhancing composability.
  - Incorporates FHE-powered modules using the TFHE-rs library for computations on encrypted data.

- **Data Availability Interface**:
  - Integrates with various data availability solutions (e.g., Celestia, Avail).
  - Compatible with mainstream Layer 1 blockchains for settlement layers.

- **zkVM Interface**:
  - Supports optimistic, zero-knowledge, and verifiable FHE virtual machines.
  - Compatible with various zkVMs, including RISC Zero and SP1.
  - Produces succinct verifiable proofs for transaction executions.

- **Threshold Service Network**:
  - Secure key management for FHE keys.
  - Robust FHE key generation and threshold decryption using MPC protocols.

<p align="center">
 <img src="assets/TMC_architecture.png" alt="TMC architecture"/>
    <br>
    <em>The Architecture of TMC</em>
</p>

## Use Cases

- **DeFi**:
  - **Dark Pools**: Enable private large trades to reduce market impact.
  - **Blind Auctions**: Conduct auctions with hidden bids to prevent manipulation.
  - **MEV-Resistant DEXs**: Build exchanges where transactions can't be front-run.
  - **Enhanced Privacy**: Improve transaction confidentiality beyond current blockchain capabilities.

- **Social Applications**:
  - **Efficient Identity Verification**: Perform identity checks without constant off-chain data retrieval.
  - **Privacy-Preserving Interactions**: Ensure all user interactions remain private.

- **Gaming**:
  - **Real-Time Response**: Enable near real-time transaction responses in distributed systems.
  - **Secure Interactions**: Operate nodes in a "dark forest" state for enhanced security.
  - **Asset Integration**: Flexible combination of DeFi and GameFi assets.
  - **Flexible Gas Fees**: Implement dynamic gas fee structures to lower entry barriers.

<p align="center">
 <img src="assets/TMC_flow.png" alt="TMC flow"/>
    <br>
    <em>The Workflow of TMC</em>
</p>

## Getting Started

Explore our demo at [beta.cyferio.com](https://beta.cyferio.com) and watch our [demo video](https://www.youtube.com/watch?v=iYxvFWpbi2s).

Feel free to explore and contribute to the project. For any questions or issues, please open an issue or contact the maintainers.
