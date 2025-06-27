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
    let mut segment_counts: HashMap<([f64; 2], [f64; 2]), u32> = HashMap::new();

    for file_bytes in files.iter() {
        let array = js_sys::Uint8Array::new(&file_bytes);
        let bytes = array.to_vec();

        if let Ok(gpx) = read(Cursor::new(bytes)) {
            for track in gpx.tracks {
                for segment in track.segments {
                    let points: Vec<[f64; 2]> = segment.points.iter()
                        .map(|p| [round(p.point().lat()), round(p.point().lon())])
                        .collect();

                    for w in points.windows(2) {
                        if let [a, b] = w {
                            let key = if a < b { (*a, *b) } else { (*b, *a) };
                            *segment_counts.entry(key).or_insert(0) += 1;
                        }
                    }
                }
            }
        }
    }

    let segments: Vec<Segment> = segment_counts.into_iter().map(|((a, b), c)| Segment {
        start: a,
        end: b,
        count: c,
    }).collect();

    JsValue::from_serde(&segments).unwrap()
}

fn round(x: f64) -> f64 {
    (x * 1e5).round() / 1e5
}
