# Stock CLI

Stock CLI is a Ratatui-based terminal assistant for A-share screening: it captures real-time snapshots, renders rotating multi-range K-line charts, and loads markets from declarative descriptors so new regions can be added by dropping CSV/JSON assets alongside the binary.

![Filtered list and K-line](./img/list_and_K-line.png)

Learn more about the full feature set:
- [English README](./docs/README_en.md)
- [中文说明](./docs/README_zh.md)

## Deployment Options
- **Build from source** – install the stable Rust toolchain, run `cargo build --release`, and ship the resulting binary together with the `assets/` directory.
- **Use the packaged release** – download the prebuilt archive, unpack it, run `./deploy.sh` (or `./deploy.sh /path/to/stock-cli`) once to clear macOS quarantine, then launch `stock-cli` directly.
