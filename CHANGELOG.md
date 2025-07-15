# Changelog

All notable changes to this project will be documented in this file.
The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

## [1.0.0-alpha.1] - 2025-07-15

### Added
- **Automated Release Pipeline:** Configured GitHub Actions to automatically build and push Docker images to GHCR and create GitHub Releases upon new tag pushes.
- **Pre-commit Hooks:** Introduced pre-commit hooks (`rustfmt`, `clippy`, `cargo-check`, and general checks) to enforce code quality and consistency.
- **Backend:** API for creating, listing, and soft-deleting tasks.
- **Frontend:** Add some JavaScript examples: client and dashboard.
- **Container Build:** Implemented a multi-stage Dockerfile for the backend, resulting in a smaller and more secure production image.

### Security
- No known security vulnerabilities addressed in this release.
