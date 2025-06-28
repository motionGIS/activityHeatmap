use wasm_bindgen::prelude::*;
use gpx::read;
use std::io::Cursor;
use serde::Serialize;
use std::collections::HashMap;

#[derive(Serialize)]
struct HeatmapTrack {
    coordinates: Vec<[f64; 2]>,
    frequency: u32,
}

#[derive(Serialize)]
struct HeatmapResult {
    tracks: Vec<HeatmapTrack>,
    max_frequency: u32,
}

#[wasm_bindgen]
pub fn process_gpx_files(files: js_sys::Array) -> JsValue {
    let mut all_tracks: Vec<Vec<[f64; 2]>> = Vec::new();
    
    // Parse all GPX files and extract tracks
    for file_bytes in files.iter() {
        let array = js_sys::Uint8Array::new(&file_bytes);
        let bytes = array.to_vec();

        if let Ok(gpx) = read(Cursor::new(bytes)) {
            for track in gpx.tracks {
                for segment in track.segments {
                    let mut track_coords = Vec::new();
                    
                    for point in segment.points {
                        track_coords.push([round(point.point().y()), round(point.point().x())]);
                    }
                    
                    if track_coords.len() > 1 {
                        // Less aggressive simplification to preserve track shape
                        let simplified = simplify_track(&track_coords, 0.00005);
                        if simplified.len() > 1 {
                            all_tracks.push(simplified);
                        }
                    }
                }
            }
        }
    }
    
    // Create a segment usage map to count overlapping segments
    let mut segment_usage: HashMap<String, u32> = HashMap::new();
    
    // Break each track into segments and count usage
    for track in &all_tracks {
        for window in track.windows(2) {
            if let [start, end] = window {
                let segment_key = create_segment_key(*start, *end);
                *segment_usage.entry(segment_key).or_insert(0) += 1;
            }
        }
    }
    
    // Calculate frequency for each track based on its segments
    let mut heatmap_tracks = Vec::new();
    let mut max_frequency = 0;
    
    for track in all_tracks {
        if track.len() < 2 {
            continue;
        }
        
        // Calculate track frequency as the average frequency of its segments
        let mut total_usage = 0;
        let mut segment_count = 0;
        
        for window in track.windows(2) {
            if let [start, end] = window {
                let segment_key = create_segment_key(*start, *end);
                if let Some(&usage) = segment_usage.get(&segment_key) {
                    total_usage += usage;
                    segment_count += 1;
                }
            }
        }
        
        // Use average usage, with minimum of 1
        let track_frequency = if segment_count > 0 {
            (total_usage as f64 / segment_count as f64).round() as u32
        } else {
            1
        };
        
        max_frequency = max_frequency.max(track_frequency);
        
        heatmap_tracks.push(HeatmapTrack {
            coordinates: track,
            frequency: track_frequency,
        });
    }
    
    let result = HeatmapResult {
        tracks: heatmap_tracks,
        max_frequency,
    };
    
    serde_wasm_bindgen::to_value(&result).unwrap()
}

fn create_segment_key(start: [f64; 2], end: [f64; 2]) -> String {
    // Use a larger tolerance for less aggressive matching
    let tolerance = 0.001; // About 100 meters
    let snap_start = snap_to_grid(start, tolerance);
    let snap_end = snap_to_grid(end, tolerance);
    
    // Normalize direction (smaller coordinate first)
    let (p1, p2) = if (snap_start[0], snap_start[1]) < (snap_end[0], snap_end[1]) {
        (snap_start, snap_end)
    } else {
        (snap_end, snap_start)
    };
    
    format!("{:.4},{:.4}-{:.4},{:.4}", p1[0], p1[1], p2[0], p2[1])
}

fn snap_to_grid(point: [f64; 2], tolerance: f64) -> [f64; 2] {
    [
        (point[0] / tolerance).round() * tolerance,
        (point[1] / tolerance).round() * tolerance,
    ]
}

fn simplify_track(points: &[[f64; 2]], tolerance: f64) -> Vec<[f64; 2]> {
    if points.len() <= 2 {
        return points.to_vec();
    }
    
    let mut result = vec![points[0]];
    let mut last_added = 0;
    
    for i in 1..points.len() {
        let distance = distance(points[last_added], points[i]);
        
        // Add point if it's far enough from the last added point
        // or if it's the last point in the track
        if distance > tolerance || i == points.len() - 1 {
            result.push(points[i]);
            last_added = i;
        }
    }
    
    result
}

fn distance(p1: [f64; 2], p2: [f64; 2]) -> f64 {
    let dx = p1[0] - p2[0];
    let dy = p1[1] - p2[1];
    (dx * dx + dy * dy).sqrt()
}

fn round(x: f64) -> f64 {
    (x * 1e5).round() / 1e5
}
