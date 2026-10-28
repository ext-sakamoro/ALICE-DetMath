//! `f64` kernels ported from fdlibm.
//!
//! `s_atan.c` / `e_atan2.c` / `e_asin.c` / `e_acos.c` / `k_sin.c` /
//! `k_cos.c` / `e_exp.c` / `e_log.c`, used directly as the double-precision
//! entry points and as the evaluation core of the `f32` inverse-trigonometric
//! functions; `sin` / `cos` over the whole range (musl `sin.c` / `cos.c` with
//! the Payne–Hanek reduction of `__rem_pio2_large.c`) are in the `trig` submodule.

use crate::ops::{round64, sqrt64};

mod pow;
mod trig;
pub use trig::{cos64, sin64, sin_cos64, tan64};

// fdlibm s_atan.c
// fdlibm's atan(0.5) / atan(1) / atan(1.5) / atan(inf) high parts; the π/4
// and π/2 entries are exactly `core::f64::consts::{FRAC_PI_4, FRAC_PI_2}`
// (same f64 bits), spelled via the constants to keep `approx_constant` quiet.
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
    // fdlibm aT[2]; the value here was mistranscribed until 0.4.0 (it made
    // atan 1898 ulp wrong just below the 0.4375 branch threshold)
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

/// Deterministic `atan(x)` in double precision (fdlibm `s_atan.c`).
#[inline]
#[must_use]
pub fn atan64(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    let ax = x.abs();
    if ax >= 7.378_697_629_483_820_646_6e19 {
        // |x| >= 2^66: atan(x) = ±π/2 (the tiny term of fdlibm rounds away)
        return if x > 0.0 {
            ATANHI[3] + ATANLO[3]
        } else {
            -ATANHI[3] - ATANLO[3]
        };
    }
    let (id, t): (i32, f64) = if ax < 0.4375 {
        if ax < 7.450_580_596_923_828_125e-9 {
            // |x| < 2^-27
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
    let s1 = z * (AT[0] + w * (AT[2] + w * (AT[4] + w * (AT[6] + w * (AT[8] + w * AT[10])))));
    let s2 = w * (AT[1] + w * (AT[3] + w * (AT[5] + w * (AT[7] + w * AT[9]))));
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

const PI_64: f64 = core::f64::consts::PI;
const PI_LO_64: f64 = 1.224_646_799_147_353_207_2e-16;
const PIO2_HI_64: f64 = core::f64::consts::FRAC_PI_2;
const PIO2_LO_64: f64 = 6.123_233_995_736_766_035_87e-17;
const PIO4_HI_64: f64 = core::f64::consts::FRAC_PI_4;

/// Deterministic `atan2(y, x)` in double precision (fdlibm `e_atan2.c`).
#[inline]
#[must_use]
pub fn atan2_64(y: f64, x: f64) -> f64 {
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    if x == 1.0 {
        return atan64(y);
    }
    // m: bit0 = sign(y), bit1 = sign(x)
    let m = u32::from(y.is_sign_negative()) | (u32::from(x.is_sign_negative()) << 1);
    if y == 0.0 {
        return match m {
            0 | 1 => y,  // atan(±0, +anything) = ±0
            2 => PI_64,  // atan(+0, -anything) = π
            _ => -PI_64, // atan(-0, -anything) = -π
        };
    }
    if x == 0.0 {
        return if y < 0.0 { -PIO2_HI_64 } else { PIO2_HI_64 };
    }
    if x.is_infinite() {
        if y.is_infinite() {
            return match m {
                0 => PIO4_HI_64,
                1 => -PIO4_HI_64,
                2 => 3.0 * PIO4_HI_64,
                _ => -3.0 * PIO4_HI_64,
            };
        }
        return match m {
            0 => 0.0,
            1 => -0.0,
            2 => PI_64,
            _ => -PI_64,
        };
    }
    if y.is_infinite() {
        return if y < 0.0 { -PIO2_HI_64 } else { PIO2_HI_64 };
    }
    let ex = ((x.to_bits() >> 52) & 0x7ff) as i32;
    let ey = ((y.to_bits() >> 52) & 0x7ff) as i32;
    let k = ey - ex;
    let z = if k > 60 {
        PIO2_HI_64 + 0.5 * PI_LO_64
    } else if x < 0.0 && k < -60 {
        0.0
    } else {
        atan64((y / x).abs())
    };
    match m {
        0 => z,
        1 => -z,
        2 => PI_64 - (z - PI_LO_64),
        _ => (z - PI_LO_64) - PI_64,
    }
}

// fdlibm e_asin.c / e_acos.c
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

#[inline(always)]
fn asin_pq(t: f64) -> (f64, f64) {
    let p = t * (PS0 + t * (PS1 + t * (PS2 + t * (PS3 + t * (PS4 + t * PS5)))));
    let q = 1.0 + t * (QS1 + t * (QS2 + t * (QS3 + t * QS4)));
    (p, q)
}

/// `x` with the low 32 bits of its mantissa cleared (fdlibm `SET_LOW_WORD(w, 0)`).
#[inline(always)]
fn clear_low_word(x: f64) -> f64 {
    f64::from_bits(x.to_bits() & 0xffff_ffff_0000_0000)
}

/// Deterministic `asin(x)` in double precision (fdlibm `e_asin.c`); `NaN`
/// outside `[-1, 1]`.
#[inline]
#[must_use]
pub fn asin64(x: f64) -> f64 {
    // NaN must not reach the arithmetic below: NaN *propagation* (which
    // payload / sign survives `1.0 - NaN`) is platform-dependent, only an
    // explicitly constructed NaN is not (wasm vs aarch64 differ here).
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
        let (p, q) = asin_pq(t);
        let w = p / q;
        return x + x * w;
    }
    let w = 1.0 - ax;
    let t = w * 0.5;
    let (p, q) = asin_pq(t);
    let s = sqrt64(t);
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

/// Deterministic `acos(x)` in double precision (fdlibm `e_acos.c`); `NaN`
/// outside `[-1, 1]`.
#[inline]
#[must_use]
pub fn acos64(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    let ax = x.abs();
    if ax >= 1.0 {
        if x == 1.0 {
            return 0.0;
        }
        if x == -1.0 {
            return PI_64 + 2.0 * PIO2_LO_64;
        }
        return f64::NAN;
    }
    if ax < 0.5 {
        if ax < 6.938_893_903_907_228_377_6e-18 {
            // |x| < 2^-57
            return PIO2_HI_64 + PIO2_LO_64;
        }
        let z = x * x;
        let (p, q) = asin_pq(z);
        let r = p / q;
        return PIO2_HI_64 - (x - (PIO2_LO_64 - x * r));
    }
    if x < 0.0 {
        // |x| >= 0.5 and negative (fdlibm branches on the sign here, so x == -0.5 lands here)
        let z = (1.0 + x) * 0.5;
        let (p, q) = asin_pq(z);
        let s = sqrt64(z);
        let r = p / q;
        let w = r * s - PIO2_LO_64;
        return PI_64 - 2.0 * (s + w);
    }
    let z = (1.0 - x) * 0.5;
    let s = sqrt64(z);
    let df = clear_low_word(s);
    let c = (z - df * df) / (s + df);
    let (p, q) = asin_pq(z);
    let r = p / q;
    let w = r * s + c;
    2.0 * (df + w)
}

// fdlibm k_sin.c / k_cos.c kernels on |r| <= π/4, and a Cody–Waite
// reduction with the 33-bit / 33-bit / 53-bit split of π/2 (e_rem_pio2.c),
// exact for |x| < 2^20·π/2.
pub(crate) const S1: f64 = -1.666_666_666_666_663_243_48e-01;
pub(crate) const S2: f64 = 8.333_333_333_322_489_461_24e-03;
pub(crate) const S3: f64 = -1.984_126_982_985_794_931_34e-04;
pub(crate) const S4: f64 = 2.755_731_370_707_006_767_89e-06;
pub(crate) const S5: f64 = -2.505_076_025_340_686_341_95e-08;
pub(crate) const S6: f64 = 1.589_690_995_211_550_102_21e-10;
pub(crate) const C1: f64 = 4.166_666_666_666_660_190_37e-02;
pub(crate) const C2: f64 = -1.388_888_888_887_410_957_49e-03;
pub(crate) const C3: f64 = 2.480_158_728_947_672_941_78e-05;
pub(crate) const C4: f64 = -2.755_731_435_139_066_330_35e-07;
pub(crate) const C5: f64 = 2.087_572_321_298_174_827_90e-09;
pub(crate) const C6: f64 = -1.135_964_755_778_819_482_65e-11;
const PIO2_1_64: f64 = 1.570_796_326_734_125_614_17e+00;
const PIO2_1T_64: f64 = 6.077_100_506_506_192_249_32e-11;
const PIO2_2_64: f64 = 6.077_100_506_303_965_976_60e-11;
const PIO2_2T_64: f64 = 2.022_266_248_795_950_631_54e-21;

/// Reduce `x` to `(k mod 4, r)` with `x = k·π/2 + r`, `|r| ≤ π/4`, in double.
#[inline(always)]
pub(crate) fn reduce_pio2_64(x: f64) -> (i32, f64) {
    let kf = round64(x * core::f64::consts::FRAC_2_PI);
    let r = if kf.abs() < 1_048_576.0 {
        (x - kf * PIO2_1_64) - kf * PIO2_1T_64
    } else {
        ((x - kf * PIO2_1_64) - kf * PIO2_2_64) - kf * PIO2_2T_64
    };
    ((kf as i32) & 3, r)
}

// ---------------------------------------------------------------------------
// exp / ln / pow
// ---------------------------------------------------------------------------

const LN2_HI64: f64 = 6.931_471_803_691_238_164_90e-01;
const LN2_LO64: f64 = 1.908_214_929_270_587_700_02e-10;
const INV_LN2_64: f64 = core::f64::consts::LOG2_E;

// fdlibm `e_exp.c`
const P1: f64 = 1.666_666_666_666_660_190_37e-01;
const P2: f64 = -2.777_777_777_701_559_338_42e-03;
const P3: f64 = 6.613_756_321_437_934_361_17e-05;
const P4: f64 = -1.653_390_220_546_525_153_90e-06;
const P5: f64 = 4.138_136_797_057_238_460_39e-08;

/// Build `2^k` for `-1022 ≤ k ≤ 1023` directly from the exponent bits.
#[inline(always)]
fn pow2i64(k: i32) -> f64 {
    debug_assert!((-1022..=1023).contains(&k));
    f64::from_bits(((k + 1023) as u64) << 52)
}

/// Deterministic `exp(x)` in double precision (fdlibm algorithm).
#[inline]
#[must_use]
pub fn exp64(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    if x > 709.782_712_893_384 {
        return f64::INFINITY;
    }
    if x < -745.133_219_101_941_1 {
        return 0.0;
    }
    let kf = round64(x * INV_LN2_64);
    let k = kf as i32;
    let hi = x - kf * LN2_HI64;
    let lo = kf * LN2_LO64;
    let r = hi - lo;
    let t = r * r;
    let c = r - t * (P1 + t * (P2 + t * (P3 + t * (P4 + t * P5))));
    let y = 1.0 - ((lo - (r * c) / (2.0 - c)) - hi);
    if k > 1023 {
        y * pow2i64(1023) * pow2i64(k - 1023)
    } else if k < -1022 {
        y * pow2i64(-1022) * pow2i64(k + 1022)
    } else {
        y * pow2i64(k)
    }
}

// fdlibm `e_log.c`
const LG1_64: f64 = 6.666_666_666_666_735_130e-01;
const LG2_64: f64 = 3.999_999_999_940_941_908e-01;
const LG3_64: f64 = 2.857_142_874_366_239_149e-01;
const LG4_64: f64 = 2.222_219_843_214_978_396e-01;
const LG5_64: f64 = 1.818_357_216_161_805_012e-01;
const LG6_64: f64 = 1.531_383_769_920_937_332e-01;
const LG7_64: f64 = 1.479_819_860_511_658_591e-01;

/// Deterministic natural logarithm in double precision (fdlibm algorithm).
///
/// `ln64(0) = -inf`, `ln64(x < 0) = NaN`, `ln64(inf) = inf`.
#[inline]
#[must_use]
pub fn ln64(x: f64) -> f64 {
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
        let scaled = x * 18_014_398_509_481_984.0; // 2^54
        bits = scaled.to_bits();
        k -= 54;
    }
    k += ((bits >> 52) as i32) - 1023;
    bits = (bits & 0x000f_ffff_ffff_ffff) | 0x3ff0_0000_0000_0000; // m in [1, 2)
    if bits >= 0x3ff6_a09e_667f_3bcd {
        // m >= sqrt(2)
        bits -= 0x0010_0000_0000_0000;
        k += 1;
    }
    let m = f64::from_bits(bits);
    let f = m - 1.0;
    let s = f / (2.0 + f);
    let z = s * s;
    let w = z * z;
    let t1 = w * (LG2_64 + w * (LG4_64 + w * LG6_64));
    let t2 = z * (LG1_64 + w * (LG3_64 + w * (LG5_64 + w * LG7_64)));
    let r = t2 + t1;
    let hfsq = 0.5 * f * f;
    let kf = f64::from(k);
    kf * LN2_HI64 - ((hfsq - (s * (hfsq + r) + kf * LN2_LO64)) - f)
}

// fdlibm `e_log10.c` base-conversion constants, and `1/ln 2` split into a
// 24-bit head plus tail (fdlibm `e_pow.c`'s `ivln2_h` / `ivln2_l`), so the
// constant itself contributes no rounding error to the product.
/// `1/ln 10`
const IVLN10_64: f64 = 4.342_944_819_032_518_166_68e-01;
/// `log10(2)`, high part (its low 32 mantissa bits are zero)
const LOG10_2HI_64: f64 = 3.010_299_956_636_117_713_06e-01;
/// `log10(2) − LOG10_2HI_64`
const LOG10_2LO_64: f64 = 3.694_239_077_158_930_899_06e-13;
/// `1/ln 2`, high part (24 bits)
const IVLN2_H_64: f64 = 1.442_695_021_629_333_496_09e+00;
/// `1/ln 2 − IVLN2_H_64`
const IVLN2_L_64: f64 = 1.925_962_991_126_617_468_87e-08;

/// `(k, mantissa bits)` with `x = 1.mantissa · 2^k` for finite `x > 0`
/// (the exponent split of fdlibm `e_log.c` / `e_log10.c`; subnormals are
/// scaled by the exact `2^54` first so the extraction stays exact).
#[inline(always)]
fn exponent_split64(x: f64) -> (i32, u64) {
    let mut bits = x.to_bits();
    let mut k: i32 = 0;
    if bits < 0x0010_0000_0000_0000 {
        let scaled = x * 18_014_398_509_481_984.0; // 2^54
        bits = scaled.to_bits();
        k -= 54;
    }
    k += ((bits >> 52) as i32) - 1023;
    (k, bits & 0x000f_ffff_ffff_ffff)
}

/// `(y, m)` with `x = m·2^y`: `m ∈ [1, 2)` for `y ≥ 0` and `m ∈ [0.5, 1)` for
/// `y < 0` (fdlibm `e_log10.c`). Keeping `m` below 1 when the exponent is
/// negative is what avoids the cancellation of `y + log(m)` around `x = 1`.
#[inline(always)]
fn log_reduce64(k: i32, mantissa: u64) -> (f64, f64) {
    let i = i32::from(k < 0);
    let m = f64::from_bits(mantissa | (((1023 - i) as u64) << 52));
    (f64::from(k + i), m)
}

/// Deterministic base-2 logarithm in double precision.
///
/// `y + log2(m)` with `x = m·2^y` (the exponent split of fdlibm `e_log10.c`),
/// the fraction from [`ln64`] converted with `1/ln 2` split into a 24-bit head
/// and a tail. A power of two returns its exponent exactly; `log2_64(0) =
/// -inf`, `log2_64(x < 0) = NaN`, `log2_64(inf) = inf`, and a NaN input
/// returns the canonical NaN.
#[inline]
#[must_use]
pub fn log2_64(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return f64::NEG_INFINITY;
    }
    if x.is_infinite() {
        return f64::INFINITY;
    }
    let (k, mantissa) = exponent_split64(x);
    let (y, m) = log_reduce64(k, mantissa);
    let t = ln64(m);
    // A power of two needs no special case: `m` is then 1 (`ln64` gives 0) or
    // 1/2, and the split constant is such that
    // `IVLN2_H_64·ln64(0.5) + IVLN2_L_64·ln64(0.5)` is exactly −1, so `y`
    // carries the answer unchanged. A guarded fast path here would be a
    // branch no input can distinguish; the exactness is pinned by the test
    // over all 2098 powers of two instead.
    y + (IVLN2_H_64 * t + IVLN2_L_64 * t)
}

/// Deterministic base-10 logarithm in double precision (fdlibm `e_log10.c`).
///
/// `y·log10(2) + log10(m)` with `x = m·2^y`, `log10(2)` split into a high and
/// a low part and the fraction taken from [`ln64`]. `log10_64(1) = 0`
/// exactly; `log10_64(0) = -inf`, `log10_64(x < 0) = NaN`, `log10_64(inf) =
/// inf`, and a NaN input returns the canonical NaN.
#[inline]
#[must_use]
pub fn log10_64(x: f64) -> f64 {
    if x.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return f64::NEG_INFINITY;
    }
    if x.is_infinite() {
        return f64::INFINITY;
    }
    let (k, mantissa) = exponent_split64(x);
    let (y, m) = log_reduce64(k, mantissa);
    let z = y * LOG10_2LO_64 + IVLN10_64 * ln64(m);
    z + y * LOG10_2HI_64
}

/// Deterministic `x^y` in double precision: fdlibm `e_pow.c` (FreeBSD msun),
/// in the `pow` submodule.
///
/// `log2 x` is formed as a two-word value and `y·log2 x` in simulated extra
/// precision before the exponential, so the result is within 1 ulp of the
/// correctly rounded value for every pair of finite arguments, including `x`
/// next to 1 with a large `|y|` (measured against an independent 2400-bit
/// reference, `tests/data/powf64_reference.txt`). Every special case is
/// IEEE 754 `pow` (C99 Annex F.10.4.4), the same rules as [`crate::powf`]:
/// `pow(x, ±0) = 1` and `pow(+1, y) = 1` for every `x` / `y` (NaN included),
/// a negative base with an integer `y` takes the sign from its parity and with
/// a non-integer finite `y` is NaN, and `±0` / `-inf` keep their sign for an
/// odd integer `y`. A NaN result is the canonical NaN.
#[inline]
#[must_use]
pub fn powf64(x: f64, y: f64) -> f64 {
    pow::pow(x, y)
}

/// Knuth's two-sum: `a + b = s + e` exactly, `s = fl(a + b)`
#[inline(always)]
fn two_sum64(a: f64, b: f64) -> (f64, f64) {
    let s = a + b;
    let bb = s - a;
    (s, (a - (s - bb)) + (b - bb))
}

/// Dekker's exact product `a·b = p + e`, `p = fl(a·b)`, with Veltkamp's split of each factor into two
/// halves of at most 26 bits (every partial product is exact in `f64`) and no fused multiply-add.
/// Exact for `|a|, |b| < 2^996` (the split multiplies by `2^27 + 1`).
#[inline(always)]
fn two_prod64(a: f64, b: f64) -> (f64, f64) {
    const SPLIT: f64 = 134_217_729.0; // 2^27 + 1
    let split = |v: f64| {
        let c = SPLIT * v;
        let h = c - (c - v);
        (h, v - h)
    };
    let p = a * b;
    let (ah, al) = split(a);
    let (bh, bl) = split(b);
    (p, (((ah * bh - p) + ah * bl) + al * bh) + al * bl)
}

/// `e^x − 1` without the cancellation of `exp64(x) − 1` near 0: fdlibm `s_expm1.c` (reduction
/// `x = k·ln2 + r` with `ln2` in two parts, a rational approximation of `r` on `|r| ≤ ln2/2`, and the
/// result rebuilt from `2^k` with the correction term), within 1 ulp. Private: it is the kernel of
/// [`tanh64`] (pinned there), not part of the crate's surface.
#[allow(
    clippy::many_single_char_names,
    clippy::cast_possible_truncation,
    clippy::cast_sign_loss
)]
fn expm1_kernel(x: f64) -> f64 {
    const O_THRESHOLD: f64 = 7.097_827_128_933_839_730_96e2;
    const LN2_HI: f64 = 6.931_471_803_691_238_164_90e-1;
    const LN2_LO: f64 = 1.908_214_929_270_587_700_02e-10;
    const INVLN2: f64 = 1.442_695_040_888_963_387_00;
    const Q1: f64 = -3.333_333_333_333_313_164_28e-2;
    const Q2: f64 = 1.587_301_587_254_814_601_65e-3;
    const Q3: f64 = -7.936_507_578_674_879_424_73e-5;
    const Q4: f64 = 4.008_217_827_329_362_395_52e-6;
    const Q5: f64 = -2.010_992_181_836_243_713_26e-7;
    let hx_full = (x.to_bits() >> 32) as u32;
    let neg = hx_full >> 31 != 0;
    let hx = hx_full & 0x7fff_ffff;
    if hx >= 0x4043_687a {
        // |x| >= 56 ln2
        if x.is_nan() {
            return f64::NAN;
        }
        if hx >= 0x4086_2e42 && !neg && x > O_THRESHOLD {
            return f64::INFINITY;
        }
        if neg {
            return -1.0;
        }
    }
    let (mut x, k, c) = if hx > 0x3fd6_2e42 {
        // |x| > ln2 / 2
        let (hi, lo, k) = if hx < 0x3ff0_a2b2 {
            if neg {
                (x + LN2_HI, -LN2_LO, -1)
            } else {
                (x - LN2_HI, LN2_LO, 1)
            }
        } else {
            let k = (INVLN2 * x + if neg { -0.5 } else { 0.5 }) as i32;
            let t = f64::from(k);
            (x - t * LN2_HI, t * LN2_LO, k)
        };
        let r = hi - lo;
        (r, k, (hi - r) - lo)
    } else if hx < 0x3c90_0000 {
        // |x| < 2^-54: the result is x
        return x;
    } else {
        (x, 0, 0.0)
    };
    let hfx = 0.5 * x;
    let hxs = x * hfx;
    let r1 = 1.0 + hxs * (Q1 + hxs * (Q2 + hxs * (Q3 + hxs * (Q4 + hxs * Q5))));
    let t = 3.0 - r1 * hfx;
    let mut e = hxs * ((r1 - t) / (6.0 - x * t));
    if k == 0 {
        return x - (x * e - hxs);
    }
    e = x * (e - c) - c;
    e -= hxs;
    if k == -1 {
        return 0.5 * (x - e) - 0.5;
    }
    if k == 1 {
        return if x < -0.25 {
            -2.0 * (e - (x + 0.5))
        } else {
            1.0 + 2.0 * (x - e)
        };
    }
    let twopk = |k: i32| f64::from_bits(((0x3ff + k) as u64) << 52);
    if k <= -2 || k > 56 {
        let mut y = 1.0 - (e - x);
        y = if k == 1024 {
            y * 2.0 * f64::from_bits(0x7fe0_0000_0000_0000)
        } else {
            y * twopk(k)
        };
        return y - 1.0;
    }
    if k < 20 {
        // 1 - 2^-k
        let t = f64::from_bits(u64::from(0x3ff0_0000_u32 - (0x0020_0000_u32 >> k)) << 32);
        (t - (e - x)) * twopk(k)
    } else {
        let t = twopk(-k);
        x -= e + t;
        (x + 1.0) * twopk(k)
    }
}

/// The odd Taylor coefficients of `tanh` from `x³` to `x⁵¹` (mpmath at 200 bits, rounded to the nearest
/// double): `tanh x = x + x·z·P(z)`, `z = x²`. On `|x| ≤ 0.7` the series after these terms is below
/// `2^-60` of the result (4.4e-19 at the end of the range), and the correction `x·z·P(z)` is at most a
/// sixth of the result, so its rounding adds a fraction of an ulp.
const TANH_TAYLOR: [f64; 25] = [
    -0.3333333333333333,
    0.13333333333333333,
    -0.05396825396825397,
    0.021869488536155203,
    -0.008863235529902197,
    0.003592128036572481,
    -0.0014558343870513183,
    0.000590027440945586,
    -0.00023912911424355248,
    9.691537956929451e-05,
    -3.927832388331683e-05,
    1.5918905069328964e-05,
    -6.451689215655431e-06,
    2.6147711512907546e-06,
    -1.0597268320104654e-06,
    4.294911078273806e-07,
    -1.7406618963571648e-07,
    7.054636946400968e-08,
    -2.859136662305254e-08,
    1.1587644432798853e-08,
    -4.6962953982309016e-09,
    1.903336833931276e-09,
    -7.713933635359062e-10,
    3.126339545892087e-10,
    -1.26705769303054e-10,
];

/// Deterministic `tanh` for `f64`, within 1 ulp of the correctly rounded value (measured against a
/// dense mpmath reference, `tests/data/tanh64_reference.txt`, and a larger sweep from the same
/// generator).
///
/// Up to `|x| = 0.7` the Taylor series `TANH_TAYLOR` (`x + x·z·P(z)`, Horner in `z = x²`): an
/// `expm1` based formula there carries `expm1`'s error amplified, since `expm1(±2x)` lies a binade or more
/// above the result (past 0.7 the factor `∂t/∂e = 2/(e + 2)²` is small enough). Above it musl `tanh.c`'s `1 − 2/(t + 2)` with `t = expm1(2|x|)` from `expm1_kernel`
/// (fdlibm `s_expm1.c`), `t + 2` kept as a two-sum, the quotient's exact remainder (Dekker's product)
/// and the subtraction's own error added together so the result is rounded once; `±1` from 22 (`1 − tanh x` is below `2^-54`
/// from `x ≈ 19.06`). Odd bit for bit (`−0` included); subnormal arguments return themselves. A NaN
/// is the canonical NaN.
#[inline]
#[must_use]
pub fn tanh64(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    let ax = x.abs();
    let t = if ax >= 22.0 {
        1.0
    } else if ax > 0.7 {
        // 1 − 2/(e + 2) rounded once: the quotient's correction and the subtraction's own error are
        // added together at the end
        let e = expm1_kernel(2.0 * ax);
        let (dh, dl) = two_sum64(e, 2.0);
        let q = 2.0 / dh;
        let (p, pe) = two_prod64(q, dh);
        let q_lo = (((2.0 - p) - pe) - q * dl) / dh;
        let (s, s_err) = two_sum64(1.0, -q);
        s + (s_err - q_lo)
    } else {
        // below `2^-27` (and for subnormals) the correction is under half an ulp and `x` is returned
        let z = ax * ax;
        let mut p = TANH_TAYLOR[TANH_TAYLOR.len() - 1];
        for &c in TANH_TAYLOR[..TANH_TAYLOR.len() - 1].iter().rev() {
            p = c + z * p;
        }
        ax + ax * (z * p)
    };
    if x.is_sign_negative() {
        -t
    } else {
        t
    }
}

/// `2^k` for a normal exponent, built from its exponent bits (exact by construction): the
/// `2^±500` thresholds and `2^±600` scales of [`hypot64`]
const fn pow2(k: i64) -> f64 {
    f64::from_bits(((1023 + k) as u64) << 52)
}

/// Deterministic `hypot` for `f64`: `sqrt(x² + y²)` within 1 ulp of the correctly rounded value,
/// with no overflow or underflow in between.
///
/// An infinity wins over a NaN (IEEE 754 `hypot`); otherwise a NaN gives the canonical NaN. When
/// the larger magnitude is above `2^500` or below `2^-500`, both are scaled by `2^∓600` first;
/// a power of two scales exactly. `x² + y²` is formed exactly as a double-double (no fused
/// multiply-add) and the root [`crate::ops::sqrt64`] (IEEE, correctly rounded) is corrected once by
/// Newton's step on its exact residual.
#[inline]
#[must_use]
pub fn hypot64(x: f64, y: f64) -> f64 {
    let ax = x.abs();
    let ay = y.abs();
    if ax.is_infinite() || ay.is_infinite() {
        return f64::INFINITY;
    }
    if ax.is_nan() || ay.is_nan() {
        return f64::NAN;
    }
    let m = if ax > ay { ax } else { ay };
    if m == 0.0 {
        return 0.0;
    }
    let (scale, unscale) = if m > pow2(500) {
        (pow2(-600), pow2(600))
    } else if m < pow2(-500) {
        (pow2(600), pow2(-600))
    } else {
        (1.0, 1.0)
    };
    let (sx, sy) = (ax * scale, ay * scale);
    // x² + y² as a double-double (Dekker's products, Knuth's sum), its root corrected once by Newton's
    // step on the exact residual `s − r²`, so the result is within an ulp of correctly rounded
    let (px, ex) = two_prod64(sx, sx);
    let (py, ey) = two_prod64(sy, sy);
    let (sh, se) = two_sum64(px, py);
    let sl = se + (ex + ey);
    let r = sqrt64(sh);
    let (rr, re) = two_prod64(r, r);
    let d = ((sh - rr) - re) + sl;
    (r + d / (2.0 * r)) * unscale
}
