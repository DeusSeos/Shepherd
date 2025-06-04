# Shepherd

Configuration as Code software for managing Rancher projects, roletemplates, and projectroletemplatebindings in a GitOps workflow written in Rust. This project aims to simplify and standardize Rancher configuration management across environments, promoting automation, reproducibility, and version control.

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/DeusSeos/Shepherd)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

## Features

- Declarative configuration for Rancher projects, roletemplates, and projectroletemplatebindings
- Support for GitOps workflows
- Integration with RK-API

## Project Goals

- Enable infrastructure teams to manage Rancher environments as code
- Reduce configuration drift and manual operations
- Provide reusable components for automation pipelines

## Tech Stack

- **Language:** Rust 🦀
- **API:** Rancher v2.10
- **Main dependencies:** [rancher_client](https://crates.io/crates/rancher_client), tokio, serde, tracing, anyhow, reqwest, json-patch
- **Configuration Format:** Y(A)ML/JSON/TOML

## Getting Started

> **Note:** Project is in early development. Expect rapid iteration and breaking changes.

### Prerequisites

- Rust (1.83 or higher recommended)
- Access to a Rancher environment with API token

### Usage

To output logs make sure to set the environment variable `RUST_LOG=none,shepherd=LOG_LEVEL` where `LOG_LEVEL` is of (`DEBUG`|`TRACE`|`INFO`)

Set the config for shepherd at `~/.shepherd/config.toml`

Example:

```toml
rancher_config_path = "/Users/samuel/Documents/Kubernetes/rancher_config"
endpoint_url = "https://rancher.rd.localhost"
file_format = "json"
token = "token-kdlz3:random312random312random312r"
remote_git_url = "git@github.com:samuel/remote_config_store.git"
cluster_names = ["cluster1", "cluster2"]
# in seconds
loop_interval = 60
# in milliseconds
retry_delay = 500
branch = "main"

[auth_method]
SshKey = "/Users/samuel/.ssh/shepherd"
```

### From source

```bash
git clone https://github.com/DeusSeos/Shepherd.git
cd Shepherd
cargo build --release
```

Run the binary with `./target/release/shepherd`.

### From releases

Download the binary from [here](https://github.com/DeusSeos/Shepherd/releases)

Run the binary with `./shepherd`

### Development

#### Dependencies

After cloning the repository run `cargo build` to download the dependencies for Shepherd.

### Authors

[Dominic Chua](https://github.com/DeusSeos)

### Contributors

[Matthew Shen](https://github.com/Sariel1563)
