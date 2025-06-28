# heatmap-parse

GPS track processor for frequency-based route heatmaps from GPX, FIT, and polyline data.

## Features

- GPX file parsing
- FIT file parsing  
- Polyline decoding
- Route frequency analysis
- WebAssembly bindings

## Usage

### Rust

```rust
use heatmap_parse::{process_gpx_files, decode_polyline, process_polylines};

let files = vec![/* Vec<u8> file data */];
let result = process_gpx_files(files);

let coords = decode_polyline("_p~iF~ps|U_ulLnnqC_mqNvxq`@");

let polylines = vec!["polyline1".to_string(), "polyline2".to_string()];
let tracks = process_polylines(polylines);
```

### WebAssembly

```javascript
import init, { process_gpx_files, decode_polyline_string } from 'heatmap-parse';

await init();

const files = [/* Uint8Array buffers */];
const result = process_gpx_files(files);

const coords = decode_polyline_string("_p~iF~ps|U_ulLnnqC_mqNvxq`@");
## Building

```bash
# Native Rust
cargo build --release

# WebAssembly
wasm-pack build --target web
```

## License

MIT
