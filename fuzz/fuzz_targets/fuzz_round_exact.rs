//! `round` / `round64` are exact functions: they must agree with the
//! platform `f32::round` / `f64::round` bit for bit on every non-NaN input,
//! including the sign of zero (a signalling NaN is quieted by the hardware
//! and passed through unchanged here, so NaN only has to stay NaN).
#![no_main]

use alice_det_math::{round, round64, sqrt64};
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: (u32, u64)| {
    let x = f32::from_bits(data.0);
    let y = f64::from_bits(data.1);
    let (a, b) = (round(x), x.round());
    assert!(
        a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan()),
        "round({:#010x}) = {:#010x} vs std {:#010x}",
        x.to_bits(),
        a.to_bits(),
        b.to_bits()
    );
    let (a, b) = (round64(y), y.round());
    assert!(
        a.to_bits() == b.to_bits() || (a.is_nan() && b.is_nan()),
        "round64({:#018x}) = {:#018x} vs std {:#018x}",
        y.to_bits(),
        a.to_bits(),
        b.to_bits()
    );
    if y >= 0.0 {
        assert_eq!(sqrt64(y).to_bits(), y.sqrt().to_bits(), "sqrt64({y:e})");
    }
});
