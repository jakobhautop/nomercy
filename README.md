#NoMercy

nomercy is written in Rust and currently built directly from the repository.

Prerequisites:
  - Rust (stable)
  - cargo available in PATH

Clone the repository:
  git clone https://github.com/<your-org>/nomercy.git
  cd nomercy

Build the engine:
  cargo build --release

The binary will be located at:
  target/release/nomercy

(Optional) Install locally:
  cargo install --path .

This installs `nomercy` into:
  ~/.cargo/bin/nomercy

Verify installation:
  nomercy --help

Running nomercy locally:
  nomercy run ./your-system-adapter

During early development, it is expected to:
  - run nomercy directly from the repo
  - iterate on the engine and protocol
  - manually drive simulations before automation

Notes:
  - nomercy is intentionally a standalone CLI
  - no test framework integration is required
  - language bindings and adapter generators may be added later
  - the Rust engine is the canonical reference implementation

Recommended workflow (early days):
  - keep nomercy built in release mode
  - run long simulations manually or in tmux/screen
  - collect repro artifacts as they appear
