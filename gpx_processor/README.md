# GPX Processor

A robust GPS track processor for creating frequency-based route heatmaps from GPX, FIT, and polyline data. This library provides both native Rust functionality and WebAssembly bindings for web applications.

## Features

- **GPX file parsing**: Parse standard GPX files and extract GPS coordinates
- **FIT file parsing**: Custom parser for Garmin FIT files (binary format)
- **Polyline decoding**: Support for Google Maps polyline format
- **Route frequency analysis**: Analyze overlapping route segments and calculate usage frequency
- **Robust filtering**: Filter out unrealistic GPS jumps and invalid coordinates
- **WebAssembly support**: Use in web browsers via WASM bindings

## Usage

### Rust

```rust
use gpx_processor::{process_gpx_files, decode_polyline, process_polylines};

// Process GPX/FIT files
let files = vec![/* your file data as Vec<u8> */];
let result = process_gpx_files(files);

// Decode polyline
let coords = decode_polyline("_p~iF~ps|U_ulLnnqC_mqNvxq`@");

// Process multiple polylines
let polylines = vec!["polyline1".to_string(), "polyline2".to_string()];
let tracks = process_polylines(polylines);
```

### WebAssembly

```javascript
import init, { process_gpx_files, decode_polyline_string } from 'gpx_processor';

await init();

// Process files in browser
const files = [/* Uint8Array buffers */];
const result = process_gpx_files(files);

// Decode polyline
const coords = decode_polyline_string("_p~iF~ps|U_ulLnnqC_mqNvxq`@");
```

## Features in Detail

### GPS Data Processing
- Parses GPX waypoints and track segments
- Custom FIT file parser supporting record, lap, and session messages
- Validates coordinates and filters invalid data
- Removes unrealistic GPS jumps (>100km between consecutive points)

### Route Frequency Analysis
- Segments routes into small pieces for overlap detection
- Counts how many times each segment is used across all routes
- Returns tracks with frequency information for heatmap visualization

### Robust Error Handling
- Graceful handling of corrupted GPS data
- Continues processing when encountering bad data sections
- Provides meaningful error messages

## File Format Support

- **GPX**: Standard GPS Exchange Format
- **FIT**: Garmin Flexible and Interoperable Data Transfer format
- **Polylines**: Google Maps encoded polyline strings

## Building

### For Native Rust
```bash
cargo build --release
```

### For WebAssembly
```bash
wasm-pack build --target web
```

## License

MIT License - see LICENSE file for details.
