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
    
    // Parse all GPX and FIT files and extract tracks
    for file_bytes in files.iter() {
        let array = js_sys::Uint8Array::new(&file_bytes);
        let bytes = array.to_vec();

        // Try to parse as GPX first
        if let Ok(gpx) = read(Cursor::new(&bytes)) {
            for track in gpx.tracks {
                for segment in track.segments {
                    let mut track_coords = Vec::new();
                    
                    for point in segment.points {
                        let lat = round(point.point().y());
                        let lon = round(point.point().x());
                        
                        // Validate coordinates to prevent globe-spanning lines
                        if is_valid_coordinate(lat, lon) {
                            track_coords.push([lat, lon]);
                        }
                    }
                    
                    if track_coords.len() > 1 {
                        // Filter out tracks with unrealistic jumps
                        let filtered_coords = filter_unrealistic_jumps(&track_coords);
                        
                        if filtered_coords.len() > 1 {
                            // Less aggressive simplification to preserve track shape
                            let simplified = simplify_track(&filtered_coords, 0.00005);
                            if simplified.len() > 1 {
                                all_tracks.push(simplified);
                            }
                        }
                    }
                }
            }
        }
        // Try to parse as FIT file if GPX parsing fails
        else if is_fit_file(&bytes) {
            // Custom FIT file parser for extracting GPS coordinates
            let mut fit_parser = FitParser::new(bytes);
            let fit_coordinates = fit_parser.parse_gps_coordinates();
            
            // Apply the same validation and filtering as GPX
            if fit_coordinates.len() > 1 {
                let filtered_coords = filter_unrealistic_jumps(&fit_coordinates);
                
                if filtered_coords.len() > 1 {
                    let simplified = simplify_track(&filtered_coords, 0.00005);
                    if simplified.len() > 1 {
                        all_tracks.push(simplified);
                    }
                }
            }
        }
        // Skip files that aren't GPX or FIT
        else {
            continue;
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

fn is_valid_coordinate(lat: f64, lon: f64) -> bool {
    // Check for valid latitude and longitude ranges
    if lat < -90.0 || lat > 90.0 || lon < -180.0 || lon > 180.0 {
        return false;
    }
    
    // Check for obviously invalid coordinates (0, 0) and other common invalid values
    if (lat == 0.0 && lon == 0.0) || lat.is_nan() || lon.is_nan() || lat.is_infinite() || lon.is_infinite() {
        return false;
    }
    
    true
}

fn filter_unrealistic_jumps(coords: &[[f64; 2]]) -> Vec<[f64; 2]> {
    if coords.len() <= 1 {
        return coords.to_vec();
    }
    
    let mut filtered = vec![coords[0]];
    let max_jump_km = 100.0; // Back to 100km for stricter filtering
    let mut consecutive_bad_points = 0;
    const MAX_CONSECUTIVE_BAD: usize = 10; // Allow up to 10 consecutive bad points
    
    for i in 1..coords.len() {
        let prev = filtered.last().unwrap();
        let curr = coords[i];
        
        // Calculate approximate distance in kilometers using Haversine formula
        let distance_km = haversine_distance(prev[0], prev[1], curr[0], curr[1]);
        
        // Only add point if it's within reasonable distance from previous point
        if distance_km <= max_jump_km {
            filtered.push(curr);
            consecutive_bad_points = 0; // Reset bad point counter
        } else {
            consecutive_bad_points += 1;
            
            // If we've seen too many consecutive bad points, try to find good data ahead
            if consecutive_bad_points <= MAX_CONSECUTIVE_BAD {
                // Look ahead up to 20 points to see if we can find a reasonable continuation
                let mut found_good_continuation = false;
                for j in (i + 1)..(i + 21).min(coords.len()) {
                    let future_point = coords[j];
                    let future_distance = haversine_distance(prev[0], prev[1], future_point[0], future_point[1]);
                    
                    // If we find a reasonable point ahead, it suggests this is just a GPS glitch
                    if future_distance <= max_jump_km * 1.5 { // Allow 1.5x distance for bridging
                        found_good_continuation = true;
                        break;
                    }
                }
                
                // If no good continuation found, we might be at the end of good data
                if !found_good_continuation {
                    // Try to find any remaining good segments by continuing to filter the rest
                    for k in (i + 1)..coords.len() {
                        let remaining_point = coords[k];
                        let remaining_distance = haversine_distance(prev[0], prev[1], remaining_point[0], remaining_point[1]);
                        
                        // If we find a reasonable point, start a new segment from there
                        if remaining_distance <= max_jump_km {
                            filtered.push(remaining_point);
                            // Continue filtering from this new point
                            for m in (k + 1)..coords.len() {
                                let next_prev = filtered.last().unwrap();
                                let next_curr = coords[m];
                                let next_distance = haversine_distance(next_prev[0], next_prev[1], next_curr[0], next_curr[1]);
                                
                                if next_distance <= max_jump_km {
                                    filtered.push(next_curr);
                                }
                                // Skip points that are too far, but don't break - keep looking
                            }
                            break; // We've processed the rest of the array
                        }
                    }
                    break; // Exit the main loop as we've processed everything
                }
            } else {
                // Too many consecutive bad points - stop processing to avoid bad data
                break;
            }
            // If there is a good continuation, just skip this point and continue
        }
    }
    
    filtered
}

fn haversine_distance(lat1: f64, lon1: f64, lat2: f64, lon2: f64) -> f64 {
    let r = 6371.0; // Earth's radius in kilometers
    let d_lat = (lat2 - lat1).to_radians();
    let d_lon = (lon2 - lon1).to_radians();
    let lat1_rad = lat1.to_radians();
    let lat2_rad = lat2.to_radians();
    
    let a = (d_lat / 2.0).sin().powi(2) + lat1_rad.cos() * lat2_rad.cos() * (d_lon / 2.0).sin().powi(2);
    let c = 2.0 * a.sqrt().atan2((1.0 - a).sqrt());
    
    r * c
}

// Custom FIT file parser for extracting GPS coordinates
// FIT file format reference: https://developer.garmin.com/fit/protocol/

struct FitParser {
    data: Vec<u8>,
    pos: usize,
    message_definitions: HashMap<u8, MessageDefinition>,
}

#[derive(Clone)]
struct MessageDefinition {
    global_message_number: u16,
    fields: Vec<FieldDefinition>,
}

#[derive(Clone)]
struct FieldDefinition {
    field_def_num: u8,
    size: u8,
    base_type: u8,
}

impl FitParser {
    fn new(data: Vec<u8>) -> Self {
        Self { 
            data, 
            pos: 0,
            message_definitions: HashMap::new(),
        }
    }

    fn read_u8(&mut self) -> Option<u8> {
        if self.pos < self.data.len() {
            let val = self.data[self.pos];
            self.pos += 1;
            Some(val)
        } else {
            None
        }
    }

    fn read_u16_le(&mut self) -> Option<u16> {
        if self.pos + 1 < self.data.len() {
            let val = u16::from_le_bytes([self.data[self.pos], self.data[self.pos + 1]]);
            self.pos += 2;
            Some(val)
        } else {
            None
        }
    }

    fn read_u32_le(&mut self) -> Option<u32> {
        if self.pos + 3 < self.data.len() {
            let val = u32::from_le_bytes([
                self.data[self.pos],
                self.data[self.pos + 1],
                self.data[self.pos + 2],
                self.data[self.pos + 3],
            ]);
            self.pos += 4;
            Some(val)
        } else {
            None
        }
    }

    fn read_i32_le(&mut self) -> Option<i32> {
        if self.pos + 3 < self.data.len() {
            let val = i32::from_le_bytes([
                self.data[self.pos],
                self.data[self.pos + 1],
                self.data[self.pos + 2],
                self.data[self.pos + 3],
            ]);
            self.pos += 4;
            Some(val)
        } else {
            None
        }
    }

    fn skip(&mut self, bytes: usize) {
        self.pos = (self.pos + bytes).min(self.data.len());
    }

    fn parse_gps_coordinates(&mut self) -> Vec<[f64; 2]> {
        let mut coordinates = Vec::new();

        // Check FIT file header
        if self.data.len() < 14 {
            return coordinates;
        }

        // FIT file header (14 bytes)
        let header_size = self.read_u8().unwrap_or(0);
        if header_size < 12 {
            return coordinates;
        }

        let _protocol_version = self.read_u8().unwrap_or(0);
        let _profile_version = self.read_u16_le().unwrap_or(0);
        let data_size = self.read_u32_le().unwrap_or(0);
        
        // Check for ".FIT" signature
        let signature = [
            self.read_u8().unwrap_or(0),
            self.read_u8().unwrap_or(0),
            self.read_u8().unwrap_or(0),
            self.read_u8().unwrap_or(0),
        ];
        if signature != [b'.', b'F', b'I', b'T'] {
            return coordinates;
        }

        // Skip header CRC if present
        if header_size == 14 {
            self.skip(2);
        }

        // Calculate data end position, but also consider that some FIT files 
        // might have the data_size field incorrect, so we'll try to parse until
        // we reach the actual end of the file (minus CRC bytes)
        let header_data_end = (self.pos + data_size as usize).min(self.data.len());
        let file_data_end = self.data.len().saturating_sub(2); // Leave 2 bytes for CRC at end
        let data_end = header_data_end.max(file_data_end); // Use the larger of the two
        
        let mut consecutive_errors = 0;
        const MAX_CONSECUTIVE_ERRORS: usize = 100; // Allow more errors before giving up
        let mut processed_bytes = 0;
        let mut last_progress_pos = self.pos;

        // Parse data records - continue until we reach the end or hit too many errors
        while self.pos < data_end && self.pos < self.data.len() && self.pos + 1 < self.data.len() {
            let start_pos = self.pos;
            
            // Every 10,000 bytes, check if we're making progress
            if self.pos - last_progress_pos > 10000 {
                processed_bytes += self.pos - last_progress_pos;
                last_progress_pos = self.pos;
                
                // If we've processed a lot of data and found some coordinates, we're probably doing well
                if coordinates.len() > 100 && processed_bytes > 50000 {
                    consecutive_errors = 0; // Reset error count as we're clearly making progress
                }
            }
            
            // Ensure we have at least 1 byte to read
            if self.pos >= self.data.len() {
                break;
            }
            
            let record_header = match self.read_u8() {
                Some(header) => header,
                None => break, // End of data
            };

            let is_definition = (record_header & 0x40) != 0;
            let local_message_type = record_header & 0x0F;

            let parse_success = if is_definition {
                // Parse definition message
                match self.parse_definition_message() {
                    Some(definition) => {
                        self.message_definitions.insert(local_message_type, definition);
                        true
                    }
                    None => {
                        // Definition parsing failed, skip ahead a bit
                        false
                    }
                }
            } else {
                // Parse data message
                if let Some(definition) = self.message_definitions.get(&local_message_type).cloned() {
                    // Verify we have enough bytes for this message
                    let total_size: usize = definition.fields.iter().map(|f| f.size as usize).sum();
                    if self.pos + total_size > self.data.len() {
                        // Not enough bytes left, try to parse what we can or skip this message
                        if total_size < 1000 { // Only try if it's a reasonable size
                            self.skip(self.data.len() - self.pos); // Skip to end
                        }
                        break;
                    }
                    
                    // Look for GPS data in multiple message types
                    match definition.global_message_number {
                        20 => {
                            // Record message (primary GPS data)
                            if let Some(coord) = self.parse_record_message(&definition) {
                                if is_valid_coordinate(coord[0], coord[1]) {
                                    coordinates.push(coord);
                                }
                            }
                            true
                        }
                        19 => {
                            // Lap message (might contain GPS data)
                            if let Some(coord) = self.parse_flexible_gps_message(&definition) {
                                if is_valid_coordinate(coord[0], coord[1]) {
                                    coordinates.push(coord);
                                }
                            }
                            true
                        }
                        18 => {
                            // Session message (might contain GPS data)
                            if let Some(coord) = self.parse_flexible_gps_message(&definition) {
                                if is_valid_coordinate(coord[0], coord[1]) {
                                    coordinates.push(coord);
                                }
                            }
                            true
                        }
                        _ => {
                            // Skip other message types but don't count as error
                            let total_size: usize = definition.fields.iter().map(|f| f.size as usize).sum();
                            if total_size < 1000 && self.pos + total_size <= self.data.len() {
                                self.skip(total_size);
                            } else {
                                // Skip to end if message is too large or would overflow
                                self.skip(self.data.len() - self.pos);
                                break;
                            }
                            true
                        }
                    }
                } else {
                    // Unknown message type - this might be an error, but try to continue
                    false
                }
            };

            if parse_success {
                consecutive_errors = 0; // Reset error counter on success
            } else {
                consecutive_errors += 1;
                
                // If we can't parse this message, try to advance by a small amount and continue
                if self.pos == start_pos {
                    // We didn't advance at all, force advancement to prevent infinite loop
                    self.skip(1);
                }
                
                // Only give up if we hit way too many consecutive errors AND we haven't found much data
                if consecutive_errors >= MAX_CONSECUTIVE_ERRORS {
                    // If we have a decent amount of coordinates, maybe this is just the end of useful data
                    if coordinates.len() < 100 {
                        break; // Give up if we don't have much data
                    } else {
                        // We have good data, try to continue a bit more
                        consecutive_errors = MAX_CONSECUTIVE_ERRORS / 2; // Reset to half
                    }
                }
            }
        }

        coordinates
    }

    fn parse_definition_message(&mut self) -> Option<MessageDefinition> {
        let start_pos = self.pos;
        
        // Check we have enough bytes for the basic structure
        if self.pos + 5 > self.data.len() {
            return None;
        }
        
        self.skip(1); // reserved byte
        self.skip(1); // architecture
        let global_message_number = self.read_u16_le()?;
        let num_fields = self.read_u8()?;

        // Sanity check on number of fields
        if num_fields > 100 {
            // This seems unreasonable, likely a parsing error
            return None;
        }

        // Check we have enough bytes for all field definitions
        if self.pos + (num_fields as usize * 3) > self.data.len() {
            return None;
        }

        let mut fields = Vec::new();
        for _ in 0..num_fields {
            // Check bounds before each field
            if self.pos + 3 > self.data.len() {
                // Not enough bytes for this field definition
                return None;
            }
            
            let field_def_num = self.read_u8()?;
            let size = self.read_u8()?;
            let base_type = self.read_u8()?;
            
            // Sanity check on field size
            if size > 100 {
                // Field size seems unreasonable, likely a parsing error
                return None;
            }
            
            fields.push(FieldDefinition {
                field_def_num,
                size,
                base_type,
            });
        }

        Some(MessageDefinition {
            global_message_number,
            fields,
        })
    }

    fn parse_record_message(&mut self, definition: &MessageDefinition) -> Option<[f64; 2]> {
        let mut lat: Option<f64> = None;
        let mut lon: Option<f64> = None;

        for field in &definition.fields {
            // More defensive bounds checking
            if field.size == 0 || self.pos >= self.data.len() || self.pos + field.size as usize > self.data.len() {
                // Skip this field if we can't read it safely
                let safe_skip = (self.data.len() - self.pos).min(field.size as usize);
                self.skip(safe_skip);
                continue;
            }
            
            match field.field_def_num {
                0 => {
                    // Latitude field
                    if field.size == 4 {
                        if let Some(lat_raw) = self.read_i32_le() {
                            if lat_raw != 0x7FFFFFFF && lat_raw != 0 {
                                let lat_degrees = lat_raw as f64 * (180.0 / 2147483648.0);
                                if lat_degrees.abs() <= 90.0 {
                                    lat = Some(lat_degrees);
                                }
                            }
                        }
                    } else {
                        self.skip(field.size as usize);
                    }
                }
                1 => {
                    // Longitude field
                    if field.size == 4 {
                        if let Some(lon_raw) = self.read_i32_le() {
                            if lon_raw != 0x7FFFFFFF && lon_raw != 0 {
                                let lon_degrees = lon_raw as f64 * (180.0 / 2147483648.0);
                                if lon_degrees.abs() <= 180.0 {
                                    lon = Some(lon_degrees);
                                }
                            }
                        }
                    } else {
                        self.skip(field.size as usize);
                    }
                }
                _ => {
                    // Skip other fields
                    self.skip(field.size as usize);
                }
            }
        }

        if let (Some(lat_val), Some(lon_val)) = (lat, lon) {
            Some([round(lat_val), round(lon_val)])
        } else {
            None
        }
    }

    fn parse_flexible_gps_message(&mut self, definition: &MessageDefinition) -> Option<[f64; 2]> {
        let mut lat: Option<f64> = None;
        let mut lon: Option<f64> = None;
        let mut potential_coords = Vec::new();

        // Collect all potential coordinate values
        for field in &definition.fields {
            // More defensive bounds checking
            if field.size == 0 || self.pos >= self.data.len() || self.pos + field.size as usize > self.data.len() {
                // Skip this field if we can't read it safely
                let safe_skip = (self.data.len() - self.pos).min(field.size as usize);
                self.skip(safe_skip);
                continue;
            }
            
            if field.size == 4 {
                if let Some(value) = self.read_i32_le() {
                    if value != 0x7FFFFFFF && value != 0 {
                        let degrees = value as f64 * (180.0 / 2147483648.0);
                        // Only consider reasonable coordinate values
                        if degrees.abs() <= 180.0 {
                            potential_coords.push(degrees);
                        }
                    }
                }
            } else {
                self.skip(field.size as usize);
            }
        }

        // Try to identify lat/lon from potential coordinates
        for coord in &potential_coords {
            if coord.abs() <= 90.0 && lat.is_none() {
                lat = Some(*coord);
            } else if coord.abs() <= 180.0 && lon.is_none() && Some(*coord) != lat {
                lon = Some(*coord);
            }
        }

        if let (Some(lat_val), Some(lon_val)) = (lat, lon) {
            Some([round(lat_val), round(lon_val)])
        } else {
            None
        }
    }
}

fn is_fit_file(data: &[u8]) -> bool {
    if data.len() < 12 {
        return false;
    }
    
    // Check for FIT signature at bytes 8-11
    data[8] == b'.' && data[9] == b'F' && data[10] == b'I' && data[11] == b'T'
}
