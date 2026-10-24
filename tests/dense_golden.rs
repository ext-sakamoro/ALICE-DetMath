//! Dense output pins of every scalar function with its own arithmetic (ALICE-Law G4).
//!
//! `tests/golden.rs` pins every function over a shared grid of about 300 000 points; a
//! change of the rounding inside one kernel (a multiply-add fused, an operation
//! reassociated) moves a few outputs in a million, which that grid can miss. Measured
//! 2026-10-17: the outer-negated fma form at `exp`'s first polynomial step moves the
//! output bits of a 2 000 000-point sweep but not the shared grid's digest. Each test here
//! sweeps about 2^21 points over the domain the function's kernel actually evaluates
//! (linear where the reduction runs, geometric in value from the smallest subnormal to the
//! largest finite value, both signs) and pins a SHA-256 of the output bits, read with
//! `digest_bits::out32` / `out64` (a NaN other than the canonical one fails the test).
//!
//! A sweep cannot prove that no bit moved anywhere; it shows that the outer-negated fma
//! mutant at one polynomial step of each kernel is caught (recorded in the G4 entry of
//! `alice-law-conformance.toml`). `DET_MATH_PRINT_DENSE=1 cargo test --release --test
//! dense_golden -- --nocapture` prints the digests instead of comparing them (re-pin after
//! an intended change, golden first).

use alice_det_math as dm;
#[path = "support/digest_bits.rs"]
mod digest_bits;
use sha2::{Digest, Sha256};
use std::fmt::Write as _;

const N: u32 = 1 << 21;

fn linear32(lo: f32, hi: f32, n: u32) -> impl Iterator<Item = f32> {
    (0..n).map(move |i| lo + (hi - lo) * (i as f32 / (n - 1) as f32))
}

/// geometric in value: a linear walk over the bit patterns of the positive finite floats
fn geometric32(n: u32) -> impl Iterator<Item = f32> {
    (0..n).map(move |i| {
        f32::from_bits(
            1 + u32::try_from(u64::from(0x7f7f_fffe_u32) * u64::from(i) / u64::from(n))
                .expect("fits"),
        )
    })
}

fn linear64(lo: f64, hi: f64, n: u32) -> impl Iterator<Item = f64> {
    (0..n).map(move |i| lo + (hi - lo) * (f64::from(i) / f64::from(n - 1)))
}

fn geometric64(n: u32) -> impl Iterator<Item = f64> {
    (0..n).map(move |i| f64::from_bits(1 + 0x7fef_ffff_ffff_fffe_u64 / u64::from(n) * u64::from(i)))
}

fn both_signs32(it: impl Iterator<Item = f32>) -> impl Iterator<Item = f32> {
    it.flat_map(|x| [x, -x])
}

fn both_signs64(it: impl Iterator<Item = f64>) -> impl Iterator<Item = f64> {
    it.flat_map(|x| [x, -x])
}

struct Pin(Sha256, u64);

impl Pin {
    fn new(name: &str) -> Self {
        let mut h = Sha256::new();
        h.update(name.as_bytes());
        Self(h, 0)
    }
    fn f32(&mut self, x: f32) {
        self.0.update(digest_bits::out32(x));
        self.1 += 1;
    }
    fn f64(&mut self, x: f64) {
        self.0.update(digest_bits::out64(x));
        self.1 += 1;
    }
    fn finish(self, name: &str, want: &str) {
        assert!(
            self.1 >= u64::from(N),
            "{name}: {} outputs hashed (compared too few)",
            self.1
        );
        let got = self.0.finalize().iter().fold(String::new(), |mut s, b| {
            let _ = write!(s, "{b:02x}");
            s
        });
        if std::env::var_os("DET_MATH_PRINT_DENSE").is_some() {
            eprintln!("DENSE {name} {got}");
            return;
        }
        assert_eq!(got, want, "{name}: output bits moved over the dense sweep");
    }
}

fn unary32(name: &str, f: fn(f32) -> f32, xs: impl Iterator<Item = f32>, want: &str) {
    let mut p = Pin::new(name);
    for x in xs {
        p.f32(f(x));
    }
    p.finish(name, want);
}

fn unary64(name: &str, f: fn(f64) -> f64, xs: impl Iterator<Item = f64>, want: &str) {
    let mut p = Pin::new(name);
    for x in xs {
        p.f64(f(x));
    }
    p.finish(name, want);
}

fn trig32() -> impl Iterator<Item = f32> {
    linear32(-12.0, 12.0, N).chain(both_signs32(geometric32(N / 2)))
}

fn trig64() -> impl Iterator<Item = f64> {
    linear64(-12.0, 12.0, N).chain(both_signs64(geometric64(N / 2)))
}

#[test]
fn sin_cos_tan_f32() {
    unary32("sin", dm::sin, trig32(), DENSE_SIN);
    unary32("cos", dm::cos, trig32(), DENSE_COS);
    unary32("tan", dm::tan, trig32(), DENSE_TAN);
}

#[test]
fn exp_ln_f32() {
    unary32(
        "exp",
        dm::exp,
        linear32(-104.0, 89.0, N).chain(both_signs32(geometric32(N / 4))),
        DENSE_EXP,
    );
    let logs = || linear32(0.5, 2.0, N).chain(geometric32(N));
    unary32("ln", dm::ln, logs(), DENSE_LN);
    unary32("log2", dm::log2, logs(), DENSE_LOG2);
    unary32("log10", dm::log10, logs(), DENSE_LOG10);
}

#[test]
fn atan_asin_acos_tanh_cbrt_f32() {
    unary32(
        "atan",
        dm::atan,
        linear32(-8.0, 8.0, N).chain(both_signs32(geometric32(N / 2))),
        DENSE_ATAN,
    );
    unary32("asin", dm::asin, linear32(-1.0, 1.0, N), DENSE_ASIN);
    unary32("acos", dm::acos, linear32(-1.0, 1.0, N), DENSE_ACOS);
    unary32(
        "tanh",
        dm::tanh,
        linear32(-12.0, 12.0, N).chain(both_signs32(geometric32(N / 2))),
        DENSE_TANH,
    );
    unary32(
        "cbrt",
        dm::cbrt,
        linear32(-8.0, 8.0, N).chain(both_signs32(geometric32(N / 2))),
        DENSE_CBRT,
    );
}

#[test]
fn hypot_atan2_powf_f32() {
    let side = 1 << 11;
    let mut h = Pin::new("hypot");
    let mut a = Pin::new("atan2");
    for y in linear32(-100.0, 100.0, side) {
        for x in linear32(-100.0, 100.0, side / 2) {
            h.f32(dm::hypot(y, x));
            a.f32(dm::atan2(y, x));
        }
    }
    h.finish("hypot", DENSE_HYPOT);
    a.finish("atan2", DENSE_ATAN2);
    let mut p = Pin::new("powf");
    for x in linear32(0.0, 16.0, 1 << 11) {
        for y in linear32(-12.0, 12.0, 1 << 10) {
            p.f32(dm::powf(x, y));
        }
    }
    p.finish("powf", DENSE_POWF);
}

#[test]
fn sin_cos_tan_f64() {
    unary64("sin64", dm::sin64, trig64(), DENSE_SIN64);
    unary64("cos64", dm::cos64, trig64(), DENSE_COS64);
    unary64("tan64", dm::tan64, trig64(), DENSE_TAN64);
}

#[test]
fn exp_ln_f64() {
    unary64(
        "exp64",
        dm::exp64,
        linear64(-745.0, 709.0, N).chain(both_signs64(geometric64(N / 4))),
        DENSE_EXP64,
    );
    let logs = || linear64(0.5, 2.0, N).chain(geometric64(N));
    unary64("ln64", dm::ln64, logs(), DENSE_LN64);
    unary64("log2_64", dm::log2_64, logs(), DENSE_LOG2_64);
    unary64("log10_64", dm::log10_64, logs(), DENSE_LOG10_64);
}

#[test]
fn atan_asin_acos_powf_f64() {
    unary64(
        "atan64",
        dm::atan64,
        linear64(-8.0, 8.0, N).chain(both_signs64(geometric64(N / 2))),
        DENSE_ATAN64,
    );
    unary64("asin64", dm::asin64, linear64(-1.0, 1.0, N), DENSE_ASIN64);
    unary64("acos64", dm::acos64, linear64(-1.0, 1.0, N), DENSE_ACOS64);
    let mut a = Pin::new("atan2_64");
    let mut p = Pin::new("powf64");
    for y in linear64(-100.0, 100.0, 1 << 11) {
        for x in linear64(-100.0, 100.0, 1 << 10) {
            a.f64(dm::atan2_64(y, x));
        }
    }
    for x in linear64(0.0, 16.0, 1 << 11) {
        for y in linear64(-12.0, 12.0, 1 << 10) {
            p.f64(dm::powf64(x, y));
        }
    }
    a.finish("atan2_64", DENSE_ATAN2_64);
    p.finish("powf64", DENSE_POWF64);
}

// recorded digests (DET_MATH_PRINT_DENSE=1), alice-det-math 0.5.1 on aarch64-apple-darwin release
const DENSE_SIN: &str = "e28bfbdf54590431020fc2d754381083079502e375db3c93546e465b8f8333df";
const DENSE_COS: &str = "2df14cae436db7efbcb3e7ddb31f4123aac22a2ea71e611715b8697f250600ab";
const DENSE_TAN: &str = "f9b424606bedc6aca6d8158d8fb5b296ee749e703aaad5c9461c9d433c93d561";
const DENSE_EXP: &str = "f2f6c46a1e0714e35ad97558e1f384cd4666a201c0ffbc58d3414a2c28f80042";
const DENSE_LN: &str = "1f5d761ef78bb4010bdb6895a26ebddcc5f3c41e374e77e8556bf3e5e292f6e1";
const DENSE_LOG2: &str = "cf7094fcda874ab7a735595ea494bbd6356a53cb6b86665882d3f2c9c2285a0b";
const DENSE_LOG10: &str = "31e75d5395f36bde22097caa28995920175a10a006f39ae8d271edf6dc413335";
const DENSE_ATAN: &str = "fc1350d7fde00e8afe3438625d3f104fda50daa0f4f487b866c14f5c1c79f227";
const DENSE_ASIN: &str = "4cb3916c1bb8e594c27d50f28dd17f8807d79fbba0e6d26f2ced6bab8376e667";
const DENSE_ACOS: &str = "1ab1d918dcce5ec0de66127f9e9d6ba4b31bde9c6428b152782e8551b4816130";
const DENSE_TANH: &str = "eb6441a0506121b7faf57a21b0c4a3e26dd43dddcd3ed577f24ec4633688de30";
const DENSE_CBRT: &str = "46852b808c0717aee842fcc52beff9b6d99edee754fa15aba31021b580aec6b2";
const DENSE_HYPOT: &str = "8b3f3b34b259a5667c05de3ea88904cffd95976850c6bf98b0373aefd7f8e97e";
const DENSE_ATAN2: &str = "ce1a592f293c0d8c7c8681f739aac7d102bc62d2c04dd9dacf2e56a881739fca";
const DENSE_POWF: &str = "b693282aaa559c8bcda51b53f830b4d69abb4d15ed91b377943509d9bc08fd4e";
const DENSE_SIN64: &str = "c13e29681e8d428f21b886f921e0cc9a48bc78fee9ab33794efb820bae837067";
const DENSE_COS64: &str = "72b50a61a190545debd58dcf8ad465831347b55bc88782b431cb9918d8f2df77";
const DENSE_TAN64: &str = "d4035e8ca82890f49faa76320974cdbfa49fb0cb4d6e8f89aea6f27ef6b70e18";
const DENSE_EXP64: &str = "d6715d4b29611a8014aef5d2602c3547a4faac427e8b8d724cfe59d3cbdb98ad";
const DENSE_LN64: &str = "101154a0950575a0bcaf2fc085443ee59b425c0dd2413e9a266ffb0bac388da1";
const DENSE_LOG2_64: &str = "e7de8a14c6a16d7efea35ed504f7e1b2c2899fbc85090690f55d2c2a9df704b1";
const DENSE_LOG10_64: &str = "0e4ca38c86ff0f27daff4762f88bf70ddd9702e22b059ec9b34d62bd7b300bf2";
const DENSE_ATAN64: &str = "d43f3b8a0f5f463e94aee9e27e5bf0e7a5740d823cb3df82afed3a0e0d6fb510";
const DENSE_ASIN64: &str = "44863563774d00797cf3e10884db94c0ffc6a2dd9a174f0dfe5864a08ecee497";
const DENSE_ACOS64: &str = "0d2936c0d6975d1b52659e1a2dd4ca08ecc8912c0b7a231a23f1e4c452ad9746";
const DENSE_ATAN2_64: &str = "8b4074e4b57dad5b97ac6e77af303cef3b368e0876423f62ab3af3184b746d91";
const DENSE_POWF64: &str = "65b08c71ff36c69417e8a2b530eb808c0c061be86c631ea5f60407c9060c0ea6";

#[test]
#[should_panic(expected = "non-canonical NaN 0xffc00000")]
fn the_pin_refuses_a_non_canonical_f32_output() {
    Pin::new("t").f32(f32::from_bits(0xffc0_0000));
}

#[test]
#[should_panic(expected = "non-canonical NaN 0xfff8000000000000")]
fn the_pin_refuses_a_non_canonical_f64_output() {
    Pin::new("t").f64(f64::from_bits(0xfff8_0000_0000_0000));
}
