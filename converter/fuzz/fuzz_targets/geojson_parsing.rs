#![no_main]

use libfuzzer_sys::fuzz_target;

// Fuzz the GeoJSON line parser to ensure no panic occurs for any byte sequence.
// The function returns None for invalid input, which is the expected safe behaviour.
fuzz_target!(|data: &[u8]| {
    if let Ok(line) = std::str::from_utf8(data) {
        // next_index can be anything; use 0 as a stable value
        let _ = converter::process_geojson_line(line, 0);
    }
});
