# REScale

```
╔═══════════════════════╗
║  RESCALE DSP ENGINE   ║
║  Zero-Dependency Rust  ║
╚═══════════════════════╝
```

**A block-based audio DSP engine and mastering CLI — written from scratch with zero third-party crates.**

[![CI](https://github.com/0xjantoko/REScale/actions/workflows/ci.yml/badge.svg)](https://github.com/0xjantoko/REScale/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue)](#license)
[![Rust](https://img.shields.io/badge/rust-stable-orange)](https://www.rust-lang.org)
[![MSRV](https://img.shields.io/badge/MSRV-1.78-blue)](#quality-gates)
[![docs.rs](https://img.shields.io/docsrs/rescale-core)](https://docs.rs/rescale-core)
[![Dependencies](https://img.shields.io/badge/deps-0-success)](#zero-dependency-policy)

> ⚠️ **v0.2.0 — early, API unstable.** DSP core works but will change. See [Known Limitations →](docs/limitations.md)

---

## Quick Start

```bash
git clone https://github.com/0xjantoko/REScale && cd REScale
cargo build --release
cargo install --path crates/rescale-cli    # optional: puts `rescale_cli` on your PATH
rescale_cli input.wav -1                   # Master to -1 dBFS (default)

# Without installing, run the build output directly:
./target/release/rescale_cli input.wav -1
```

See [Quickstart Guide](docs/quickstart.md) for full usage.

---

## Navigation

| Topic | File |
|-------|------|
| 🚀 Getting Started | [docs/quickstart.md](docs/quickstart.md) |
| 🎛️ Architecture & Components | [docs/architecture.md](docs/architecture.md) |
| 💻 CLI Reference | [docs/cli-reference.md](docs/cli-reference.md) |
| 📚 Library API | [docs/api.md](docs/api.md) |
| ⚠️ Known Limitations | [docs/limitations.md](docs/limitations.md) |
| 🗺️ Roadmap | [docs/roadmap.md](docs/roadmap.md) |
| 🎧 MP3 Pipeline | [docs/mp3-rescale-guide.md](docs/mp3-rescale-guide.md) |
| 🖥️ TUI Guide | [docs/tui-guide.md](docs/tui-guide.md) |
| 📊 Competitive Analysis | [docs/project-analysis.md](docs/project-analysis.md) |
| 🏗️ Internal Docs | [docs/implementation-guide.md](docs/implementation-guide.md), [docs/math-foundation.md](docs/math-foundation.md) |

---

## Repository Layout

```
crates/
  rescale-core/   # DSP library: engine, buffer, filter, drc, fft, wav, gain,
                  #   normalize, convert, limiter, tui (zero deps, unsafe-free)
  rescale-cli/    # Mastering CLI binary + runnable examples
docs/             # All documentation guides
```

## Quality Gates

```bash
cargo fmt --all --check                              # ✅ clean
cargo clippy --workspace --all-targets --locked -- -D warnings   # ✅ clean
cargo build  --workspace --all-targets --locked                  # ✅ 0 warnings
cargo test   --workspace --all-targets --locked                  # ✅ 58/58 pass (49 core + 9 CLI)
```

**MSRV: Rust 1.78** — verified in CI. Lockfile v4.

## Maintenance

Solo project. PRs not accepted; issues welcome. Minimal reproducible report = fastest fix. Dependabot CI PRs merged directly. Security reports → [SECURITY.md](SECURITY.md).

Source comments: Indonesian. User-facing content: English.

## License

Dual-licensed under **MIT** or **Apache-2.0**.
