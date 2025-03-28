# Rancher Config-as-Code

A configuration-as-code project for managing Rancher deployments and resources programmatically using Rust. This project aims to simplify and standardize Rancher configuration management across environments, promoting automation, reproducibility, and version control.

## Features

- Declarative configuration for Rancher clusters, projects, and workloads
- Support for GitOps workflows
- CLI and library interfaces (WIP)
- Validation and dry-run modes
- Integration with RK-API

## Project Goals

- Enable infrastructure teams to manage Rancher environments as code
- Reduce configuration drift and manual operations
- Provide reusable components for automation pipelines

## Tech Stack

- **Language:** Rust 🦀
- **API:** Rancher v2.10
- **Configuration Format:** YAML/JSON (planned)

## Getting Started

> **Note:** Project is in early development. Expect rapid iteration and breaking changes.

### Prerequisites

- Rust (1.83 or higher recommended)
- Access to a Rancher environment with API token

### Build

```bash
cargo build --release
```

### Authors

[Dominic Chua](https://github.com/DeusSeos)
[Matthew Shen](https://github.com/Sariel1563)
