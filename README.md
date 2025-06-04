# Shepherd

Configuration as Code software for managing Rancher projects, roletemplates, and projectroletemplatebindings in a GitOps workflow written in Rust. This project aims to simplify and standardize Rancher configuration management across environments, promoting automation, reproducibility, and version control.

[![Ask DeepWiki](https://deepwiki.com/badge.svg)](https://deepwiki.com/DeusSeos/Shepherd)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](https://opensource.org/licenses/Apache-2.0)

## Features

- Declarative configuration for Rancher projects, roletemplates, and projectroletemplatebindings
- Support for GitOps workflows
- Validation and dry-run modes
- Integration with RK-API

## Project Goals

- Enable infrastructure teams to manage Rancher environments as code
- Reduce configuration drift and manual operations
- Provide reusable components for automation pipelines

## Tech Stack

- **Language:** Rust 🦀
- **API:** Rancher v2.10
- **Configuration Format:** Y(A)ML/JSON/TOML

## Getting Started

> **Note:** Project is in early development. Expect rapid iteration and breaking changes.



### Prerequisites

- Rust (1.83 or higher recommended)
- Access to a Rancher environment with API token

### Development

#### Dependencies

After cloning the repository run `cargo build` to download the dependencies for Shepherd.

### Authors

[Dominic Chua](https://github.com/DeusSeos)

### Contributors

[Matthew Shen](https://github.com/Sariel1563)
