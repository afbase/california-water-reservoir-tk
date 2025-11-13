# Dioxus CDEC Water Reservoir Viewer

A modern web application for visualizing California water reservoir data using **Dioxus 0.7.1**, **D3.js**, and **WASM**.

## Features

- ✅ **Dioxus 0.7.1** - Modern Rust web framework for WASM
- ✅ **D3.js** - Interactive data visualization
- ✅ **Zstd Compression** - Efficient data storage (138 KB compressed from 823 KB JSON)
- ✅ **Pure WASM** - Compiled for `wasm32-unknown-unknown` target
- ✅ **GitHub Pages Ready** - Static deployment with no backend required
- ✅ **In-Memory Data** - Compressed JSON embedded in WASM binary

## Architecture

### Data Storage
Instead of using a large LZMA blob or complex SQLite WASM setup, this implementation uses:
- **JSON data** extracted from CDEC cumulative observations
- **Zstd compression** (16.71% compression ratio)
- **`include_bytes!` macro** to embed compressed data in the WASM binary
- **In-memory decompression** on app startup

### Tech Stack
- **Frontend Framework**: Dioxus 0.7.1 (web platform)
- **Visualization**: D3.js v7
- **Data Format**: Zstd-compressed JSON
- **Build Target**: wasm32-unknown-unknown
- **Deployment**: GitHub Pages (static files)

## Building

### Prerequisites
```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Add WASM target
rustup target add wasm32-unknown-unknown

# Install Dioxus CLI
cargo install dioxus-cli --version 0.7.1
# or faster with binstall:
cargo install cargo-binstall
cargo binstall dioxus-cli --version 0.7.1
```

### Build Steps

1. **Generate compressed data** (if needed):
```bash
python3 build_json.py
```

2. **Build for release**:
```bash
dx build --release
```

3. **Output** will be in `target/dx/dioxus-cdec/release/web/public/`

## Development

```bash
dx serve
```

This will start a development server with hot-reloading.

## Project Structure

```
dioxus-cdec/
├── src/
│   ├── main.rs          # App entry point & main UI
│   ├── database.rs      # Data loading & filtering
│   └── chart.rs         # D3.js chart component
├── data/
│   ├── reservoir_data.json      # Uncompressed data (generated)
│   └── reservoir_data.json.zst  # Compressed data (embedded)
├── assets/
│   └── chart.js         # D3.js chart implementation (reference)
├── index.html           # Custom HTML template
├── Dioxus.toml          # Dioxus configuration
├── build_json.py        # Data extraction script
└── README.md            # This file
```

## Data Flow

1. **Build Time**:
   - Extract CSV from `cumulative_v2.tar.lzma`
   - Convert to JSON format
   - Compress with zstd
   - Embed in WASM binary via `include_bytes!`

2. **Runtime**:
   - Decompress JSON data in browser
   - Parse observations into memory
   - Filter by date range
   - Render with D3.js

## Performance

- **Compressed data**: 138 KB
- **Uncompressed data**: 823 KB (35,930 observations)
- **Date range**: 1925-01-01 to 2024-12-31
- **WASM bundle**: ~25 MB (includes all dependencies)

## Comparison with Previous Implementations

| Feature | Yew (old) | Dioxus (new) |
|---------|-----------|--------------|
| Framework | Yew 0.21 | Dioxus 0.7.1 |
| Visualization | Plotters | D3.js |
| Data Format | LZMA blob | Zstd JSON |
| Data Size | ~1.6 MB | 138 KB |
| Charts | SVG (server-side) | D3 (interactive) |

## License

Same as parent project
