//! ALICE-Law G4: inputs that separate one fused polynomial step from the library's two roundings.
//!
//! The library evaluates every polynomial step `c + w * v` with two roundings. A build that
//! fuses one step into `w.mul_add(v, c)` (one rounding) is the contraction G4 asks the tests to
//! see. For the inner steps of the double-precision kernels the change is a fraction of an
//! output ulp, so a random sample of a few million inputs does not find an input whose output
//! moves. This tool finds them in three stages:
//!
//! 1. Inputs are drawn only from the band where the step's variable is largest (the step's
//!    perturbation reaches the output scaled by that variable's powers).
//! 2. An error-free test at the step itself: the step's operands are recorded on the unfused
//!    path and `w.mul_add(v, c)` is compared with `c + w * v`. Only inputs where the two roundings
//!    of the step differ can change the output; the others are skipped.
//! 3. For those inputs the whole function is evaluated with the step fused and compared bit for
//!    bit with the unfused evaluation.
//!
//! The unfused kernels here are copies of the library's; every evaluated input is also compared
//! with the library function itself, and a difference stops the run (exit 1), so a copy that
//! drifts from `src/double.rs` cannot report inputs for a kernel the library no longer has.
//!
//! The inputs come from a fixed SplitMix64 sequence per step and the index ranges are split
//! across threads by position, so a run with the same arguments prints the same inputs.
//!
//! Usage: `cargo run --release -- [--log2-n N] [--keep K] [STEP ...]` (default: every step,
//! N = 30, K = 3). Prints one `found` line per kept input and one `summary` line per step.
//! `cargo run --release -- --check-pins tests/contraction_goldens.rs` checks that each pin of a
//! modelled step separates it (the library gives the pinned bits, the fused build does not).

// the fdlibm constants are copied digit for digit from src/double.rs, which keeps them so
#![allow(clippy::excessive_precision)]
// the threads split the inputs by position and their results are merged by index, so the
// order of completion does not reach the output (the repo's clippy.toml bans thread::scope
// for the library, whose results must not depend on scheduling)
#![allow(clippy::disallowed_methods)]

use alice_det_math as dm;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Step {
    None,
    P2,
    P3,
    P4,
    At4,
    At5,
    At6,
    At7,
    At8,
    Ps4,
    Lg3,
    Lg4,
    Lg5,
}

const STEPS: [(Step, &str, &str); 12] = [
    (Step::P2, "P2", "exp64"),
    (Step::P3, "P3", "exp64"),
    (Step::P4, "P4", "exp64"),
    (Step::At4, "AT4", "atan64"),
    (Step::At5, "AT5", "atan64"),
    (Step::At6, "AT6", "atan64"),
    (Step::At7, "AT7", "atan64"),
    (Step::At8, "AT8", "atan64"),
    (Step::Ps4, "PS4", "asin64"),
    (Step::Lg3, "LG3", "ln64"),
    (Step::Lg4, "LG4", "ln64"),
    (Step::Lg5, "LG5", "ln64"),
];

/// The step `me`: `c + w * v`, fused when `me == fuse`; its operands are recorded when
/// `me == record`.
struct Ctx {
    fuse: Step,
    record: Step,
    ops: Option<(f64, f64, f64)>,
}

impl Ctx {
    fn new(fuse: Step, record: Step) -> Self {
        Self {
            fuse,
            record,
            ops: None,
        }
    }
    #[inline(always)]
    fn st(&mut self, me: Step, c: f64, w: f64, v: f64) -> f64 {
        if me == self.record {
            self.ops = Some((c, w, v));
        }
        if me == self.fuse {
            w.mul_add(v, c)
        } else {
            c + w * v
        }
    }
}

// ---- copies of src/double.rs (fdlibm), each step routed through Ctx::st ----

const LN2_HI64: f64 = 6.931_471_803_691_238_164_90e-01;
const LN2_LO64: f64 = 1.908_214_929_270_587_700_02e-10;
const INV_LN2_64: f64 = core::f64::consts::LOG2_E;
const P1: f64 = 1.666_666_666_666_660_190_37e-01;
const P2: f64 = -2.777_777_777_701_559_338_42e-03;
const P3: f64 = 6.613_756_321_437_934_361_17e-05;
const P4: f64 = -1.653_390_220_546_525_153_90e-06;
const P5: f64 = 4.138_136_797_057_238_460_39e-08;

fn pow2i64(k: i32) -> f64 {
    f64::from_bits(((k + 1023) as u64) << 52)
}

fn exp64(x: f64, cx: &mut Ctx) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x > 709.782_712_893_384 {
        return f64::INFINITY;
    }
    if x < -745.133_219_101_941_1 {
        return 0.0;
    }
    let kf = dm::round64(x * INV_LN2_64);
    let k = kf as i32;
    let hi = x - kf * LN2_HI64;
    let lo = kf * LN2_LO64;
    let r = hi - lo;
    let t = r * r;
    let p4 = cx.st(Step::P4, P4, t, P5);
    let p3 = cx.st(Step::P3, P3, t, p4);
    let p2 = cx.st(Step::P2, P2, t, p3);
    let c = r - t * (P1 + t * p2);
    let y = 1.0 - ((lo - (r * c) / (2.0 - c)) - hi);
    if k > 1023 {
        y * pow2i64(1023) * pow2i64(k - 1023)
    } else if k < -1022 {
        y * pow2i64(-1022) * pow2i64(k + 1022)
    } else {
        y * pow2i64(k)
    }
}

const ATANHI: [f64; 4] = [
    4.636_476_090_008_060_935_15e-01,
    core::f64::consts::FRAC_PI_4,
    9.827_937_232_473_290_540_82e-01,
    core::f64::consts::FRAC_PI_2,
];
const ATANLO: [f64; 4] = [
    2.269_877_745_296_168_709_24e-17,
    3.061_616_997_868_383_017_93e-17,
    1.390_331_103_123_099_845_16e-17,
    6.123_233_995_736_766_035_87e-17,
];
const AT: [f64; 11] = [
    3.333_333_333_333_293_180_27e-01,
    -1.999_999_999_987_648_324_76e-01,
    1.428_571_427_250_346_637_11e-01,
    -1.111_111_040_546_235_578_80e-01,
    9.090_887_133_436_506_561_96e-02,
    -7.691_876_205_044_829_994_95e-02,
    6.661_073_137_387_531_206_69e-02,
    -5.833_570_133_790_573_486_45e-02,
    4.976_877_994_615_932_360_17e-02,
    -3.653_157_274_421_691_552_70e-02,
    1.628_582_011_536_578_236_23e-02,
];

fn atan64(x: f64, cx: &mut Ctx) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    let ax = x.abs();
    if ax >= 7.378_697_629_483_820_646_6e19 {
        return if x > 0.0 {
            ATANHI[3] + ATANLO[3]
        } else {
            -ATANHI[3] - ATANLO[3]
        };
    }
    let (id, t): (i32, f64) = if ax < 0.4375 {
        if ax < 7.450_580_596_923_828_125e-9 {
            return x;
        }
        (-1, x)
    } else if ax < 1.1875 {
        if ax < 0.6875 {
            (0, (2.0 * ax - 1.0) / (2.0 + ax))
        } else {
            (1, (ax - 1.0) / (ax + 1.0))
        }
    } else if ax < 2.4375 {
        (2, (ax - 1.5) / (1.0 + 1.5 * ax))
    } else {
        (3, -1.0 / ax)
    };
    let z = t * t;
    let w = z * z;
    let a8 = cx.st(Step::At8, AT[8], w, AT[10]);
    let a6 = cx.st(Step::At6, AT[6], w, a8);
    let a4 = cx.st(Step::At4, AT[4], w, a6);
    let s1 = z * (AT[0] + w * (AT[2] + w * a4));
    let a7 = cx.st(Step::At7, AT[7], w, AT[9]);
    let a5 = cx.st(Step::At5, AT[5], w, a7);
    let s2 = w * (AT[1] + w * (AT[3] + w * a5));
    if id < 0 {
        return t - t * (s1 + s2);
    }
    let i = id as usize;
    let r = ATANHI[i] - ((t * (s1 + s2) - ATANLO[i]) - t);
    if x < 0.0 {
        -r
    } else {
        r
    }
}

const PIO2_HI_64: f64 = core::f64::consts::FRAC_PI_2;
const PIO2_LO_64: f64 = 6.123_233_995_736_766_035_87e-17;
const PIO4_HI_64: f64 = core::f64::consts::FRAC_PI_4;
const PS0: f64 = 1.666_666_666_666_666_574_15e-01;
const PS1: f64 = -3.255_658_186_224_009_154_05e-01;
const PS2: f64 = 2.012_125_321_348_629_258_81e-01;
const PS3: f64 = -4.005_553_450_067_941_140_27e-02;
const PS4: f64 = 7.915_349_942_898_145_321_76e-04;
const PS5: f64 = 3.479_331_075_960_211_675_70e-05;
const QS1: f64 = -2.403_394_911_734_414_218_78e+00;
const QS2: f64 = 2.020_945_760_233_505_694_71e+00;
const QS3: f64 = -6.882_839_716_054_532_930_30e-01;
const QS4: f64 = 7.703_815_055_590_193_527_91e-02;

fn asin_pq(t: f64, cx: &mut Ctx) -> (f64, f64) {
    let p4 = cx.st(Step::Ps4, PS4, t, PS5);
    let p = t * (PS0 + t * (PS1 + t * (PS2 + t * (PS3 + t * p4))));
    let q = 1.0 + t * (QS1 + t * (QS2 + t * (QS3 + t * QS4)));
    (p, q)
}

fn clear_low_word(x: f64) -> f64 {
    f64::from_bits(x.to_bits() & 0xffff_ffff_0000_0000)
}

fn asin64(x: f64, cx: &mut Ctx) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    let ax = x.abs();
    if ax >= 1.0 {
        if ax == 1.0 {
            return x * PIO2_HI_64 + x * PIO2_LO_64;
        }
        return f64::NAN;
    }
    if ax < 0.5 {
        if ax < 7.450_580_596_923_828_125e-9 {
            return x;
        }
        let t = x * x;
        let (p, q) = asin_pq(t, cx);
        let w = p / q;
        return x + x * w;
    }
    let w = 1.0 - ax;
    let t = w * 0.5;
    let (p, q) = asin_pq(t, cx);
    let s = dm::sqrt64(t);
    let r = if ax >= 0.975 {
        let w = p / q;
        PIO2_HI_64 - (2.0 * (s + s * w) - PIO2_LO_64)
    } else {
        let w = clear_low_word(s);
        let c = (t - w * w) / (s + w);
        let r = p / q;
        let p = 2.0 * s * r - (PIO2_LO_64 - 2.0 * c);
        let q = PIO4_HI_64 - 2.0 * w;
        PIO4_HI_64 - (p - q)
    };
    if x < 0.0 {
        -r
    } else {
        r
    }
}

const LG1_64: f64 = 6.666_666_666_666_735_130e-01;
const LG2_64: f64 = 3.999_999_999_940_941_908e-01;
const LG3_64: f64 = 2.857_142_874_366_239_149e-01;
const LG4_64: f64 = 2.222_219_843_214_978_396e-01;
const LG5_64: f64 = 1.818_357_216_161_805_012e-01;
const LG6_64: f64 = 1.531_383_769_920_937_332e-01;
const LG7_64: f64 = 1.479_819_860_511_658_591e-01;

fn ln64(x: f64, cx: &mut Ctx) -> f64 {
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return f64::NEG_INFINITY;
    }
    if x.is_infinite() {
        return f64::INFINITY;
    }
    let mut bits = x.to_bits();
    let mut k: i32 = 0;
    if bits < 0x0010_0000_0000_0000 {
        let scaled = x * 18_014_398_509_481_984.0;
        bits = scaled.to_bits();
        k -= 54;
    }
    k += ((bits >> 52) as i32) - 1023;
    bits = (bits & 0x000f_ffff_ffff_ffff) | 0x3ff0_0000_0000_0000;
    if bits >= 0x3ff6_a09e_667f_3bcd {
        bits -= 0x0010_0000_0000_0000;
        k += 1;
    }
    let m = f64::from_bits(bits);
    let f = m - 1.0;
    let s = f / (2.0 + f);
    let z = s * s;
    let w = z * z;
    let l4 = cx.st(Step::Lg4, LG4_64, w, LG6_64);
    let t1 = w * (LG2_64 + w * l4);
    let l5 = cx.st(Step::Lg5, LG5_64, w, LG7_64);
    let l3 = cx.st(Step::Lg3, LG3_64, w, l5);
    let t2 = z * (LG1_64 + w * l3);
    let r = t2 + t1;
    let hfsq = 0.5 * f * f;
    let kf = f64::from(k);
    kf * LN2_HI64 - ((hfsq - (s * (hfsq + r) + kf * LN2_LO64)) - f)
}

// ---- the stage-2 filter: each kernel's polynomial alone ----
//
// The value the polynomial hands to the rest of the function, from the same reduction as the
// copies above. Only an input whose polynomial value differs between the two builds can
// change the output, so the full evaluations (and the comparison with the library) run only
// for those. The filter decides what is evaluated, never what is reported: every reported
// input is re-evaluated in full above.

fn poly(f: &str, x: f64, cx: &mut Ctx) -> (u64, u64) {
    match f {
        "exp64" => {
            let kf = dm::round64(x * INV_LN2_64);
            let r = (x - kf * LN2_HI64) - kf * LN2_LO64;
            let t = r * r;
            let p4 = cx.st(Step::P4, P4, t, P5);
            let p3 = cx.st(Step::P3, P3, t, p4);
            let p2 = cx.st(Step::P2, P2, t, p3);
            ((r - t * (P1 + t * p2)).to_bits(), 0)
        }
        "atan64" => {
            let ax = x.abs();
            let t = if ax < 0.4375 {
                x
            } else if ax < 0.6875 {
                (2.0 * ax - 1.0) / (2.0 + ax)
            } else if ax < 1.1875 {
                (ax - 1.0) / (ax + 1.0)
            } else if ax < 2.4375 {
                (ax - 1.5) / (1.0 + 1.5 * ax)
            } else {
                -1.0 / ax
            };
            let z = t * t;
            let w = z * z;
            let a8 = cx.st(Step::At8, AT[8], w, AT[10]);
            let a6 = cx.st(Step::At6, AT[6], w, a8);
            let a4 = cx.st(Step::At4, AT[4], w, a6);
            let s1 = z * (AT[0] + w * (AT[2] + w * a4));
            let a7 = cx.st(Step::At7, AT[7], w, AT[9]);
            let a5 = cx.st(Step::At5, AT[5], w, a7);
            let s2 = w * (AT[1] + w * (AT[3] + w * a5));
            (s1.to_bits(), s2.to_bits())
        }
        "asin64" => {
            let ax = x.abs();
            let t = if ax < 0.5 { x * x } else { (1.0 - ax) * 0.5 };
            let (p, _) = asin_pq(t, cx);
            (p.to_bits(), 0)
        }
        "ln64" => {
            let mut bits = x.to_bits();
            if bits < 0x0010_0000_0000_0000 {
                bits = (x * 18_014_398_509_481_984.0).to_bits();
            }
            bits = (bits & 0x000f_ffff_ffff_ffff) | 0x3ff0_0000_0000_0000;
            if bits >= 0x3ff6_a09e_667f_3bcd {
                bits -= 0x0010_0000_0000_0000;
            }
            let f = f64::from_bits(bits) - 1.0;
            let s = f / (2.0 + f);
            let z = s * s;
            let w = z * z;
            let l4 = cx.st(Step::Lg4, LG4_64, w, LG6_64);
            let t1 = w * (LG2_64 + w * l4);
            let l5 = cx.st(Step::Lg5, LG5_64, w, LG7_64);
            let l3 = cx.st(Step::Lg3, LG3_64, w, l5);
            let t2 = z * (LG1_64 + w * l3);
            (t1.to_bits(), t2.to_bits())
        }
        _ => unreachable!(),
    }
}

// ---- inputs ----

fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

/// A uniform double in [0, 1) from the top 53 bits.
fn unit(u: u64) -> f64 {
    (u >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
}

fn lerp(a: f64, b: f64, u: f64) -> f64 {
    a + (b - a) * u
}

/// The input of index `i` for function `f`, drawn from the band where the polynomial
/// variable is largest. Branch boundaries are those of the copies above.
fn input(f: &str, seed: u64, i: u64) -> f64 {
    let h = splitmix64(seed ^ splitmix64(i));
    let pick = h & 0xf;
    let sign = if (h >> 4) & 1 == 1 { -1.0 } else { 1.0 };
    let u = unit(splitmix64(h));
    match f {
        // |r| = |x - k ln2| in [0.25, ln2/2]: x = (k + d) ln2, |d| in [0.36, 0.5]
        "exp64" => {
            let k = ((h >> 8) % 1400) as f64 - 700.0;
            let d = lerp(0.36, 0.5, u) * sign;
            (k + d) * core::f64::consts::LN_2
        }
        // |t| near the top of a branch; the |x| < 0.4375 branch (output about t, in the binade
        // [0.25, 0.5)) gets half the draws: there the output ulp is smallest against the step
        "atan64" => {
            let ax = match pick % 8 {
                0..=3 => lerp(0.36, 0.4375, u),
                4 => lerp(0.6875, 0.75, u),
                5 => lerp(1.0, 1.1875, u),
                6 => lerp(1.9, 2.4375, u),
                _ => lerp(2.4375, 3.2, u),
            };
            ax * sign
        }
        // t = x^2 in [0.16, 0.25] (|x| < 0.5) or t = (1 - |x|) / 2 in [0.16, 0.25]
        "asin64" => {
            let ax = if pick & 1 == 0 {
                lerp(0.4, 0.5, u)
            } else {
                lerp(0.5, 0.68, u)
            };
            ax * sign
        }
        // |s| = |f / (2 + f)| near its maximum with k = 0 (a nonzero k ln 2 term enlarges the
        // output ulp): x in [1.3, sqrt 2) (f > 0) or [sqrt 2 / 2, 0.775] (f < 0, after the shift)
        "ln64" => {
            if pick & 1 == 0 {
                lerp(1.3, core::f64::consts::SQRT_2, u)
            } else {
                lerp(core::f64::consts::FRAC_1_SQRT_2, 0.775, u)
            }
        }
        _ => unreachable!(),
    }
}

fn eval(f: &str, x: f64, cx: &mut Ctx) -> f64 {
    match f {
        "exp64" => exp64(x, cx),
        "atan64" => atan64(x, cx),
        "asin64" => asin64(x, cx),
        "ln64" => ln64(x, cx),
        _ => unreachable!(),
    }
}

fn library(f: &str, x: f64) -> f64 {
    match f {
        "exp64" => dm::exp64(x),
        "atan64" => dm::atan64(x),
        "asin64" => dm::asin64(x),
        "ln64" => dm::ln64(x),
        _ => unreachable!(),
    }
}

struct Found {
    index: u64,
    x: f64,
    out: f64,
    fused: f64,
}

#[derive(Default)]
struct Tally {
    compared: u64,
    flips: u64,
    changed: u64,
    poly_changed: u64,
    library_checked: u64,
    found: Vec<Found>,
}

fn search(
    step: Step,
    f: &'static str,
    seed: u64,
    lo: u64,
    hi: u64,
    keep: usize,
    bad: &AtomicBool,
) -> Tally {
    let mut t = Tally::default();
    for i in lo..hi {
        let x = input(f, seed, i);
        t.compared += 1;
        // stage 2a, the error-free test at the step: do its two roundings differ from one?
        let mut cx = Ctx::new(Step::None, step);
        let plain = poly(f, x, &mut cx);
        let drift_probe = i % 4096 == 0;
        let Some((c, w, v)) = cx.ops else { continue };
        let step_differs = w.mul_add(v, c).to_bits() != (c + w * v).to_bits();
        if step_differs {
            t.flips += 1;
        }
        // stage 2b: does the difference survive to the polynomial's value?
        let poly_differs = step_differs && poly(f, x, &mut Ctx::new(step, Step::None)) != plain;
        if !poly_differs && !drift_probe {
            continue;
        }
        // stage 3: the whole function, both builds, and the copy against the library
        let out = eval(f, x, &mut Ctx::new(Step::None, Step::None));
        if out.to_bits() != library(f, x).to_bits() {
            eprintln!(
                "copy drift: {f}({:#018x}) = {:#018x}, library {:#018x}",
                x.to_bits(),
                out.to_bits(),
                library(f, x).to_bits()
            );
            bad.store(true, Ordering::Relaxed);
            return t;
        }
        t.library_checked += 1;
        if !poly_differs {
            continue;
        }
        t.poly_changed += 1;
        let fused = eval(f, x, &mut Ctx::new(step, Step::None));
        if fused.to_bits() != out.to_bits() {
            t.changed += 1;
            if t.found.len() < keep {
                t.found.push(Found {
                    index: i,
                    x,
                    out,
                    fused,
                });
            }
        }
    }
    t
}

/// The leading-term steps of `tests/contraction_goldens.rs` with the function each is pinned
/// through, found by an earlier search and not modelled here: a pin naming one of these pairs is
/// skipped and counted, any other pair this tool does not model (a typo in the step or the
/// function, or a modelled step under another function) fails the check
const LEADING: [(&str, &str); 10] = [
    ("exp64", "P1"),
    ("atan64", "AT0"),
    ("atan64", "AT1"),
    ("atan64", "AT3"),
    ("asin64", "PS0"),
    ("asin64", "QS1"),
    ("acos64", "PS0"),
    ("acos64", "QS1"),
    ("ln64", "LG2"),
    ("log10_64", "LG1"),
];

/// The result of checking the pins of one file
#[derive(Debug, Default, PartialEq, Eq)]
struct PinReport {
    checked: u32,
    skipped: u32,
    unknown: u32,
    bad: u32,
}

impl PinReport {
    /// Passes when at least one pin was checked and none failed or named an unknown step
    fn passes(&self) -> bool {
        self.checked > 0 && self.bad == 0 && self.unknown == 0
    }
}

/// Checks every `("function", "step", 0x…, 0x…)` pin in `text`: a pin of a step this tool models
/// must give the pinned bits from the library and other bits with that step fused; a pin of a
/// leading step is skipped; any other step name is unknown
fn check_pins_text(text: &str) -> PinReport {
    let flat: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    let mut r = PinReport::default();
    for chunk in flat.split("(\"").skip(1) {
        // name","step",0x…,0x…)
        let Some((name, rest)) = chunk.split_once("\",\"") else {
            continue;
        };
        let Some((step_name, rest)) = rest.split_once("\",") else {
            continue;
        };
        let mut nums = rest.split(',').take(2).map(|t| {
            let t = t.trim_end_matches(')').replace('_', "");
            t.strip_prefix("0x")
                .and_then(|h| u64::from_str_radix(h, 16).ok())
        });
        let (Some(Some(xb)), Some(Some(want))) = (nums.next(), nums.next()) else {
            continue;
        };
        let Some(&(step, _, f)) = STEPS.iter().find(|(_, n, f)| *n == step_name && *f == name)
        else {
            if LEADING.contains(&(name, step_name)) {
                r.skipped += 1;
            } else {
                r.unknown += 1;
                println!("pin {name} {step_name} x={xb:#018x}: a step this tool neither models nor lists as leading");
            }
            continue;
        };
        let x = f64::from_bits(xb);
        let lib = library(f, x).to_bits();
        let fused = eval(f, x, &mut Ctx::new(step, Step::None)).to_bits();
        r.checked += 1;
        if lib != want || fused == want {
            r.bad += 1;
            println!("pin {name} {step_name} x={xb:#018x}: library {lib:#018x}, fused {fused:#018x}, pinned {want:#018x}");
        }
    }
    r
}

/// `--check-pins FILE`: every pin of `tests/contraction_goldens.rs` whose step this tool models
/// must give the pinned bits from the library and other bits with that step fused, so the named
/// step is the one the pin separates. A pin of a leading step (`LEADING`) is skipped and counted;
/// an unknown step name, a pin that does not separate its step, and checking none fail.
fn check_pins(path: &str) -> i32 {
    let text = std::fs::read_to_string(path).unwrap_or_else(|e| panic!("{path}: {e}"));
    let r = check_pins_text(&text);
    println!(
        "check-pins: {} pins checked, {} of leading steps not modelled here, {} of unknown steps, {} not separating",
        r.checked, r.skipped, r.unknown, r.bad
    );
    if r.passes() {
        0
    } else {
        1
    }
}

fn main() {
    let mut log2_n = 30u32;
    let mut keep = 3usize;
    let mut chosen: Vec<String> = Vec::new();
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--log2-n" => {
                log2_n = args
                    .next()
                    .and_then(|s| s.parse().ok())
                    .expect("--log2-n N")
            }
            "--keep" => keep = args.next().and_then(|s| s.parse().ok()).expect("--keep K"),
            "--check-pins" => {
                std::process::exit(check_pins(&args.next().expect("--check-pins FILE")))
            }
            s => chosen.push(s.to_string()),
        }
    }
    let n = 1u64 << log2_n;
    let threads = thread::available_parallelism().map_or(4, |p| p.get()) as u64;
    let bad = AtomicBool::new(false);
    let mut steps_run = 0;
    for (k, &(step, name, f)) in STEPS.iter().enumerate() {
        if !chosen.is_empty() && !chosen.iter().any(|c| c == name) {
            continue;
        }
        steps_run += 1;
        let seed = 0x6334_0000 + k as u64;
        let parts: Vec<Tally> = thread::scope(|s| {
            let hs: Vec<_> = (0..threads)
                .map(|j| {
                    let (lo, hi) = (n * j / threads, n * (j + 1) / threads);
                    let bad = &bad;
                    s.spawn(move || search(step, f, seed, lo, hi, keep, bad))
                })
                .collect();
            hs.into_iter().map(|h| h.join().unwrap()).collect()
        });
        if bad.load(Ordering::Relaxed) {
            std::process::exit(1);
        }
        let compared: u64 = parts.iter().map(|t| t.compared).sum();
        let flips: u64 = parts.iter().map(|t| t.flips).sum();
        let changed: u64 = parts.iter().map(|t| t.changed).sum();
        let poly_changed: u64 = parts.iter().map(|t| t.poly_changed).sum();
        let library_checked: u64 = parts.iter().map(|t| t.library_checked).sum();
        let mut found: Vec<Found> = parts.into_iter().flat_map(|t| t.found).collect();
        found.sort_by_key(|f| f.index);
        for fd in found.iter().take(keep) {
            println!(
                "found {f} {name} index={} x={:#018x} out={:#018x} fused={:#018x}",
                fd.index,
                fd.x.to_bits(),
                fd.out.to_bits(),
                fd.fused.to_bits()
            );
        }
        println!("summary {f} {name} drawn={compared} step_differs={flips} poly_differs={poly_changed} output_changed={changed} library_checked={library_checked}");
        if compared == 0 || library_checked == 0 {
            eprintln!("nothing compared for {name}");
            std::process::exit(1);
        }
    }
    if steps_run == 0 {
        eprintln!("no step selected");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // an AT4 pin of tests/contraction_goldens.rs (tools/contraction-search --log2-n 30 AT4)
    const X: u64 = 0x3fda_92d7_72f0_e9e3;
    const AT4: &str = r#"("atan64", "AT4", 0x3fda92d772f0e9e3, 0x3fd92ff252ff3c88),"#;

    fn pin(fun: &str, step: &str, x: u64, out: u64) -> String {
        format!("(\"{fun}\", \"{step}\", {x:#018x}, {out:#018x}),")
    }

    fn fused_at4(x: f64) -> u64 {
        eval("atan64", x, &mut Ctx::new(Step::At4, Step::None)).to_bits()
    }

    #[test]
    fn a_pin_of_a_modelled_step_is_checked() {
        let r = check_pins_text(AT4);
        assert_eq!(
            r,
            PinReport {
                checked: 1,
                ..PinReport::default()
            }
        );
        assert!(r.passes());
    }

    #[test]
    fn a_pin_with_bits_neither_build_gives_fails() {
        let x = f64::from_bits(X);
        let other = 0x3fd9_2ff2_52ff_3c89;
        assert_ne!(other, dm::atan64(x).to_bits());
        assert_ne!(other, fused_at4(x));
        let r = check_pins_text(&pin("atan64", "AT4", X, other));
        assert_eq!((r.checked, r.bad), (1, 1));
        assert!(!r.passes());
    }

    #[test]
    fn a_pin_whose_input_does_not_separate_its_step_fails() {
        // the library's own bits at an input where the fused step gives the same output
        let x = 0.4_f64;
        let lib = dm::atan64(x).to_bits();
        assert_eq!(lib, fused_at4(x), "the input must not separate AT4");
        let r = check_pins_text(&pin("atan64", "AT4", x.to_bits(), lib));
        assert_eq!((r.checked, r.bad), (1, 1));
        assert!(!r.passes());
    }

    #[test]
    fn an_unknown_step_name_fails_even_beside_a_good_pin() {
        for typo in [
            AT4.replace("\"AT4\"", "\"AT9\""),
            AT4.replace("\"atan64\"", "\"atan\""),
            r#"("atan", "AT0", 0xc0046ab2f098b14c, 0xbff32857868822b0),"#.to_string(),
        ] {
            let r = check_pins_text(&typo);
            assert_eq!((r.checked, r.unknown), (0, 1), "{typo}");
            assert!(!r.passes());
            let r = check_pins_text(&format!("{AT4}\n{typo}"));
            assert_eq!((r.checked, r.unknown, r.bad), (1, 1, 0), "{typo}");
            assert!(!r.passes(), "a good pin beside {typo}");
        }
    }

    #[test]
    fn a_leading_step_is_skipped_and_checking_none_fails() {
        let r =
            check_pins_text(r#"("exp64", "P1", 0x403e_1e55_1477_9197, 0x42a5_e23f_5657_3e26),"#);
        assert_eq!(
            r,
            PinReport {
                skipped: 1,
                ..PinReport::default()
            }
        );
        assert!(!r.passes());
        assert!(!check_pins_text("").passes());
    }
}
