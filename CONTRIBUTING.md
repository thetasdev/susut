# Contributing to susut

Thanks for helping! Bug reports, ideas and pull requests are all welcome.
Please follow the [Code of Conduct](CODE_OF_CONDUCT.md).

## Before you start

- For a bug, open an issue with the command you ran and the output. If a file
  triggers it, attach a small sample **only if it is safe to share**. Never
  upload photos of people or identity documents.
- For a new feature, open an issue first so we can agree on the design.
- For a security problem, do **not** open a public issue; see [SECURITY.md](SECURITY.md).

## Development setup

You need Rust 1.88 or newer and a C compiler. On x86_64 you also need
[`nasm`](https://www.nasm.us/) (see the README).

```sh
git clone https://github.com/thetasdev/susut
cd susut
cargo test
```

Before opening a pull request, these must pass:

```sh
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test
cargo deny check      # licenses and advisories (cargo install cargo-deny)
```

## Code layout

- `src/lib.rs`: decoding, resizing, encoding, the size-target search, parsing. No terminal output.
- `src/message.rs`: user-facing runtime messages in English and Indonesian.
- `src/main.rs`: argument parsing, collecting files, printing.
- `tests/cli.rs`: end-to-end tests that run the binary on generated fixtures.

## Guidelines

- Add or update tests with every behavior change. Fixtures must be generated in
  code or be tiny (under 20 KB) and free of copyright and personal data.
- Every new user-facing message needs both an English and an Indonesian version
  in `src/message.rs`.
- Changing existing CLI behavior is a breaking change: say so in the pull request
  and add it to `CHANGELOG.md` under *Unreleased*.
- New dependencies need a license compatible with `MIT OR Apache-2.0`. GPL/AGPL
  crates (for example `imagequant`) are not accepted. State the license in the PR.
- Keep the MSRV in `Cargo.toml` honest. Raise it only when needed and note it in the changelog.

## License

By contributing you agree that your work is licensed under `MIT OR Apache-2.0`, like the rest of the project.
