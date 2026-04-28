# Contributing to partly-claudy

Thanks for your interest. partly-claudy is maintained by [TAHO Engineering](https://taho.is).

## Getting Started

1. Fork the repository
2. Clone your fork and create a branch
3. Make your changes
4. Run the CI gate locally before pushing:

   ```sh
   cargo fmt --check
   cargo clippy --all-targets -- -D warnings
   cargo test
   ```
5. Open a pull request

## Reporting Issues

Open issues on GitHub Issues with clear descriptions.

## Discussions

GitHub Discussions for questions and ideas.

## Style

- Rust 2024 edition
- `cargo sort` for Cargo.toml formatting
- Follow existing code patterns

## License

Dual licensed MIT OR Apache-2.0. Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this work by you shall be dual licensed as above, without any additional terms or conditions.
