//! `f64` kernels ported from fdlibm.
//!
//! `s_atan.c` / `e_atan2.c` / `e_asin.c` / `e_acos.c` / `k_sin.c` /
//! `k_cos.c` / `e_exp.c` / `e_log.c`, used directly as the double-precision
//! entry points and as the evaluation core of the `f32` inverse-trigonometric
//! functions.

use crate::ops::{round64, sqrt64};

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
    1.428_571_427_593_712_314_80e-01,
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
const S1: f64 = -1.666_666_666_666_663_243_48e-01;
const S2: f64 = 8.333_333_333_322_489_461_24e-03;
const S3: f64 = -1.984_126_982_985_794_931_34e-04;
const S4: f64 = 2.755_731_370_707_006_767_89e-06;
const S5: f64 = -2.505_076_025_340_686_341_95e-08;
const S6: f64 = 1.589_690_995_211_550_102_21e-10;
const C1: f64 = 4.166_666_666_666_660_190_37e-02;
const C2: f64 = -1.388_888_888_887_410_957_49e-03;
const C3: f64 = 2.480_158_728_947_672_941_78e-05;
const C4: f64 = -2.755_731_435_139_066_330_35e-07;
const C5: f64 = 2.087_572_321_298_174_827_90e-09;
const C6: f64 = -1.135_964_755_778_819_482_65e-11;
const PIO2_1_64: f64 = 1.570_796_326_734_125_614_17e+00;
const PIO2_1T_64: f64 = 6.077_100_506_506_192_249_32e-11;
const PIO2_2_64: f64 = 6.077_100_506_303_965_976_60e-11;
const PIO2_2T_64: f64 = 2.022_266_248_795_950_631_54e-21;

/// fdlibm `k_sin`: `sin(x)` for `|x| ≤ π/4`.
#[inline(always)]
pub(crate) fn k_sin64(x: f64) -> f64 {
    let z = x * x;
    let v = z * x;
    let r = S2 + z * (S3 + z * (S4 + z * (S5 + z * S6)));
    x + v * (S1 + z * r)
}

/// fdlibm `k_cos`: `cos(x)` for `|x| ≤ π/4`.
#[inline(always)]
pub(crate) fn k_cos64(x: f64) -> f64 {
    let z = x * x;
    let r = z * (C1 + z * (C2 + z * (C3 + z * (C4 + z * (C5 + z * C6)))));
    let hz = 0.5 * z;
    let w = 1.0 - hz;
    w + ((1.0 - w) - hz + (z * r))
}

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

/// Exact product `a·b = p + e` (Dekker / Veltkamp splitting, no `fma`).
#[inline(always)]
fn two_prod(a: f64, b: f64) -> (f64, f64) {
    const SPLIT: f64 = 134_217_729.0; // 2^27 + 1
    let p = a * b;
    let ta = SPLIT * a;
    let a_hi = ta - (ta - a);
    let a_lo = a - a_hi;
    let tb = SPLIT * b;
    let b_hi = tb - (tb - b);
    let b_lo = b - b_hi;
    let e = ((a_hi * b_hi - p) + a_hi * b_lo + a_lo * b_hi) + a_lo * b_lo;
    (p, e)
}

/// Deterministic `x^y` in double precision, same special-case rules as
/// [`crate::powf`].
///
/// `y · ln x` is formed as a double-double (`ln64` plus a one-step residual
/// correction, exact product via Dekker splitting) so the exponential's
/// argument error is not amplified by `|y · ln x|`; measured ≤ 13 ulp over
/// the documented domain (the residual is limited by `exp64`'s own rounding).
#[must_use]
pub fn powf64(x: f64, y: f64) -> f64 {
    if y == 0.0 {
        return 1.0;
    }
    if x.is_nan() || y.is_nan() || x < 0.0 {
        return f64::NAN;
    }
    if x == 0.0 {
        return if y > 0.0 { 0.0 } else { f64::INFINITY };
    }
    if x.is_infinite() {
        return if y > 0.0 { f64::INFINITY } else { 0.0 };
    }
    // ln x = l_hi + l_lo: residual of one exp round trip recovers the low part.
    let l_hi = ln64(x);
    let l_lo = x * exp64(-l_hi) - 1.0;
    let (t_hi, mut t_lo) = two_prod(y, l_hi);
    t_lo += y * l_lo;
    if t_hi > 709.782_712_893_384 {
        return f64::INFINITY;
    }
    if t_hi < -745.133_219_101_941_1 {
        return 0.0;
    }
    exp64(t_hi) * (1.0 + t_lo)
}
