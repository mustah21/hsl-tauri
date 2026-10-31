#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use libfuzzer_sys::fuzz_target;

#[derive(Arbitrary, Debug)]
struct Input {
    value: f64,
    min: f64,
    max: f64,
}

// Fuzz the coordinate quantizer to verify it never panics for any f64 triple,
// including NaN, infinity, subnormal, and denormalised values.
// A u16 result is always produced — no range checks should cause panics.
fuzz_target!(|data: &[u8]| {
    let mut u = Unstructured::new(data);
    if let Ok(input) = Input::arbitrary(&mut u) {
        let q = converter::quantize(input.value, input.min, input.max);
        // Result must always be a valid u16 (0–65535)
        assert!(q <= u16::MAX);
    }
});
