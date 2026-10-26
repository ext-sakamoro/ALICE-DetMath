//! The error function, `erf(x) = 2/√π ∫₀ˣ e^{-t²} dt`.
//!
//! [`erf64`] is fdlibm `s_erf.c` (rational approximations on four ranges of
//! `|x|`, the tails through `exp(-x² - 0.5625 + R/S)/x`) with the
//! exponentials taken from [`exp64`], so every operation is an IEEE 754 basic
//! operation or a function of this crate and the result is the same bits on
//! every target. [`erf`] is [`erf64`] rounded once to `f32`.
//!
//! Accuracy (`tests/erf_accuracy.rs`, against correctly rounded values from
//! `mpmath` at 2400 bits, `scripts/gen_erf_reference.py`): [`erf64`] ≤ 1 ulp
//! over 6492 inputs, [`erf`] ≤ 1 ulp, measured max 0 over 7250 inputs; every
//! branch of the algorithm is measured.

use crate::double::exp64;

/// `erf(1)` rounded to single precision (`c` of the `[0.84375, 1.25]` branch).
const ERX: f64 = 8.450_629_115_104_675_292_97e-01;
/// `2/√π - 1`, for `|x| < 2^-28`.
const EFX8: f64 = 1.027_033_336_764_100_690_53e+00;
// erf on [0, 0.84375]: x + x·P(x²)/Q(x²)
const PP0: f64 = 1.283_791_670_955_125_585_61e-01;
const PP1: f64 = -3.250_421_072_470_014_993_70e-01;
const PP2: f64 = -2.848_174_957_559_851_047_66e-02;
const PP3: f64 = -5.770_270_296_489_441_591_57e-03;
const PP4: f64 = -2.376_301_665_665_016_260_84e-05;
const QQ1: f64 = 3.979_172_239_591_553_528_19e-01;
const QQ2: f64 = 6.502_224_998_876_729_444_85e-02;
const QQ3: f64 = 5.081_306_281_875_765_627_76e-03;
const QQ4: f64 = 1.324_947_380_043_216_445_26e-04;
const QQ5: f64 = -3.960_228_278_775_368_123_20e-06;
// erf on [0.84375, 1.25]: c + P1(s)/Q1(s), s = |x| - 1
const PA0: f64 = -2.362_118_560_752_659_440_77e-03;
const PA1: f64 = 4.148_561_186_837_483_316_66e-01;
const PA2: f64 = -3.722_078_760_357_013_238_47e-01;
const PA3: f64 = 3.183_466_199_011_617_536_74e-01;
const PA4: f64 = -1.108_946_942_823_966_774_76e-01;
const PA5: f64 = 3.547_830_432_561_823_593_71e-02;
const PA6: f64 = -2.166_375_594_868_790_843_00e-03;
const QA1: f64 = 1.064_208_804_008_442_282_86e-01;
const QA2: f64 = 5.403_979_177_021_710_489_37e-01;
const QA3: f64 = 7.182_865_441_419_626_628_68e-02;
const QA4: f64 = 1.261_712_198_087_616_421_12e-01;
const QA5: f64 = 1.363_708_391_202_905_073_62e-02;
const QA6: f64 = 1.198_449_984_679_910_741_70e-02;
/// fdlibm's `1/0.35` switch: the high word `0x4006_db6d` with a zero low word.
const ERFC_SWITCH: f64 = 2.857_141_494_750_976_6;
// erfc on [1.25, 1/0.35]: exp(-x² - 0.5625 + R1(z)/S1(z)) / x, z = 1/x²
const RA0: f64 = -9.864_944_034_847_148_227_05e-03;
const RA1: f64 = -6.938_585_727_071_817_643_72e-01;
const RA2: f64 = -1.055_862_622_532_329_098_14e+01;
const RA3: f64 = -6.237_533_245_032_600_603_96e+01;
const RA4: f64 = -1.623_966_694_625_734_703_55e+02;
const RA5: f64 = -1.846_050_929_067_110_359_94e+02;
const RA6: f64 = -8.128_743_550_630_659_342_46e+01;
const RA7: f64 = -9.814_329_344_169_145_485_92e+00;
const SA1: f64 = 1.965_127_166_743_925_712_92e+01;
const SA2: f64 = 1.376_577_541_435_190_426_00e+02;
const SA3: f64 = 4.345_658_774_752_292_288_21e+02;
const SA4: f64 = 6.453_872_717_332_678_803_36e+02;
const SA5: f64 = 4.290_081_400_275_678_333_86e+02;
const SA6: f64 = 1.086_350_055_417_794_351_34e+02;
const SA7: f64 = 6.570_249_770_319_281_701_35e+00;
const SA8: f64 = -6.042_441_521_485_809_874_38e-02;
// erfc on [1/0.35, 6]: the same form with R2 / S2
const RB0: f64 = -9.864_942_924_700_099_285_97e-03;
const RB1: f64 = -7.992_832_376_805_230_065_74e-01;
const RB2: f64 = -1.775_795_491_775_475_198_89e+01;
const RB3: f64 = -1.606_363_848_558_219_160_62e+02;
const RB4: f64 = -6.375_664_433_683_896_277_22e+02;
const RB5: f64 = -1.025_095_131_611_077_249_54e+03;
const RB6: f64 = -4.835_191_916_086_513_970_19e+02;
const SB1: f64 = 3.033_806_074_348_245_829_24e+01;
const SB2: f64 = 3.257_925_129_965_739_188_26e+02;
const SB3: f64 = 1.536_729_586_084_436_959_94e+03;
const SB4: f64 = 3.199_858_219_508_595_539_08e+03;
const SB5: f64 = 2.553_050_406_433_164_425_83e+03;
const SB6: f64 = 4.745_285_412_069_553_672_15e+02;
const SB7: f64 = -2.244_095_244_658_581_833_62e+01;

/// `erfc(|x|)` for `0.84375 ≤ |x| < 6`.
#[inline(always)]
fn erfc_mid(ax: f64) -> f64 {
    if ax < 1.25 {
        let s = ax - 1.0;
        let p = PA0 + s * (PA1 + s * (PA2 + s * (PA3 + s * (PA4 + s * (PA5 + s * PA6)))));
        let q = 1.0 + s * (QA1 + s * (QA2 + s * (QA3 + s * (QA4 + s * (QA5 + s * QA6)))));
        return 1.0 - ERX - p / q;
    }
    let s = 1.0 / (ax * ax);
    let (r, big_s) = if ax < ERFC_SWITCH {
        (
            RA0 + s * (RA1 + s * (RA2 + s * (RA3 + s * (RA4 + s * (RA5 + s * (RA6 + s * RA7)))))),
            1.0 + s
                * (SA1
                    + s * (SA2
                        + s * (SA3 + s * (SA4 + s * (SA5 + s * (SA6 + s * (SA7 + s * SA8))))))),
        )
    } else {
        (
            RB0 + s * (RB1 + s * (RB2 + s * (RB3 + s * (RB4 + s * (RB5 + s * RB6))))),
            1.0 + s * (SB1 + s * (SB2 + s * (SB3 + s * (SB4 + s * (SB5 + s * (SB6 + s * SB7)))))),
        )
    };
    // -x² = -z² + (z - x)(z + x) with z = x truncated to its high 32 bits, so
    // z² is exact and the large part of the exponent carries no rounding.
    let z = f64::from_bits(ax.to_bits() & 0xffff_ffff_0000_0000);
    exp64(-z * z - 0.5625) * exp64((z - ax) * (z + ax) + r / big_s) / ax
}

/// Deterministic error function `erf(x) = 2/√π ∫₀ˣ e^{-t²} dt` in double
/// precision (fdlibm `s_erf.c`, with [`exp64`] for the exponentials).
///
/// `erf64(±inf) = ±1`, `erf64(NaN) = NaN`, `erf64(-0) = -0`; `|x| ≥ 6` returns
/// `±1` (the true value is within `2^-53` of it from `|x| ≈ 5.92`).
#[inline]
#[must_use]
pub fn erf64(x: f64) -> f64 {
    if x.is_nan() {
        return f64::NAN;
    }
    let ax = x.abs();
    if ax < 0.84375 {
        if ax < 3.725_290_298_461_914e-9 {
            // |x| < 2^-28: erf(x) = x + (2/√π - 1)·x, scaled to avoid underflow
            return 0.125 * (8.0 * x + EFX8 * x);
        }
        let z = x * x;
        let r = PP0 + z * (PP1 + z * (PP2 + z * (PP3 + z * PP4)));
        let s = 1.0 + z * (QQ1 + z * (QQ2 + z * (QQ3 + z * (QQ4 + z * QQ5))));
        return x + x * (r / s);
    }
    let y = if ax < 6.0 { 1.0 - erfc_mid(ax) } else { 1.0 };
    if x < 0.0 {
        -y
    } else {
        y
    }
}

/// Deterministic error function in single precision: [`erf64`] evaluated
/// in double precision and rounded once (≤ 1 ulp).
///
/// `erf(±inf) = ±1`, `erf(NaN) = NaN`, `erf(-0) = -0`.
#[inline]
#[must_use]
pub fn erf(x: f32) -> f32 {
    erf64(f64::from(x)) as f32
}
