use wasm_bindgen::prelude::*;
use gpx::read;
use std::collections::HashMap;
use std::io::Cursor;
use serde::Serialize;

#[derive(Serialize)]
struct Segment {
    start: [f64; 2],
    end: [f64; 2],
    count: u32,
}

#[wasm_bindgen]
pub fn process_gpx_files(files: js_sys::Array) -> JsValue {
    let mut segment_counts: HashMap<(String, String), u32> = HashMap::new();

    for file_bytes in files.iter() {
        let array = js_sys::Uint8Array::new(&file_bytes);
        let bytes = array.to_vec();

        if let Ok(gpx) = read(Cursor::new(bytes)) {
            for track in gpx.tracks {
                for segment in track.segments {
                    let points: Vec<[f64; 2]> = segment.points.iter()
                        .map(|p| [round(p.point().y()), round(p.point().x())])
                        .collect();

                    for w in points.windows(2) {
                        if let [a, b] = w {
                            let key_a = format!("{},{}", a[0], a[1]);
                            let key_b = format!("{},{}", b[0], b[1]);
                            let key = if key_a < key_b { (key_a, key_b) } else { (key_b, key_a) };
                            *segment_counts.entry(key).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    let segments: Vec<Segment> = segment_counts.into_iter().map(|((a, b), c)| {
        let start_coords: Vec<f64> = a.split(',').map(|s| s.parse().unwrap()).collect();
        let end_coords: Vec<f64> = b.split(',').map(|s| s.parse().unwrap()).collect();
        Segment {
            start: [start_coords[0], start_coords[1]],
            end: [end_coords[0], end_coords[1]],
            count: c,
        }
    }).collect();

    serde_wasm_bindgen::to_value(&segments).unwrap()
}

fn round(x: f64) -> f64 {
    (x * 1e5).round() / 1e5
}
