//! `powf64` (fdlibm `e_pow.c`, `src/double/pow.rs`) and `erf64` (musl `erf.c`, `src/erf.rs`)
//! with every polynomial step `c + w * v` routed through [`C::st`], for the same search as the
//! kernels of `main.rs`: the step is fused into `w.mul_add(v, c)` when it is the one under test
//! and its operands are recorded when it is the one being tested for a differing rounding.
//!
//! The copies are generated from the library sources (the polynomial lines rewritten step by
//! step, nothing else changed) and every evaluated input is compared with the library, so a
//! copy that drifts stops the search.

// the constants are copied digit for digit from the library, which keeps them so
#![allow(
    clippy::excessive_precision,
    clippy::unreadable_literal,
    clippy::approx_constant
)]

use alice_det_math as dm;

/// One polynomial step of `powf64` or `erf64`.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum S {
    None,
    PowL5,
    PowL4,
    PowL3,
    PowL2,
    PowL1,
    PowP4,
    PowP3,
    PowP2,
    PowP1,
    ErfPa5,
    ErfPa4,
    ErfPa3,
    ErfPa2,
    ErfPa1,
    ErfPa0,
    ErfQa5,
    ErfQa4,
    ErfQa3,
    ErfQa2,
    ErfQa1,
    ErfQa0,
    ErfRa6,
    ErfRa5,
    ErfRa4,
    ErfRa3,
    ErfRa2,
    ErfRa1,
    ErfRa0,
    ErfSa7,
    ErfSa6,
    ErfSa5,
    ErfSa4,
    ErfSa3,
    ErfSa2,
    ErfSa1,
    ErfSa0,
    ErfRb5,
    ErfRb4,
    ErfRb3,
    ErfRb2,
    ErfRb1,
    ErfRb0,
    ErfSb6,
    ErfSb5,
    ErfSb4,
    ErfSb3,
    ErfSb2,
    ErfSb1,
    ErfSb0,
    ErfPp3,
    ErfPp2,
    ErfPp1,
    ErfPp0,
    ErfQq4,
    ErfQq3,
    ErfQq2,
    ErfQq1,
    ErfQq0,
}

/// `(step, pin name, function)`; the pin name is the coefficient the step adds
pub const STEPS: &[(S, &str, &str)] = &[
    (S::PowL5, "L5", "powf64"),
    (S::PowL4, "L4", "powf64"),
    (S::PowL3, "L3", "powf64"),
    (S::PowL2, "L2", "powf64"),
    (S::PowL1, "L1", "powf64"),
    (S::PowP4, "P4", "powf64"),
    (S::PowP3, "P3", "powf64"),
    (S::PowP2, "P2", "powf64"),
    (S::PowP1, "P1", "powf64"),
    (S::ErfPa5, "PA5", "erf64"),
    (S::ErfPa4, "PA4", "erf64"),
    (S::ErfPa3, "PA3", "erf64"),
    (S::ErfPa2, "PA2", "erf64"),
    (S::ErfPa1, "PA1", "erf64"),
    (S::ErfPa0, "PA0", "erf64"),
    (S::ErfQa5, "QA5", "erf64"),
    (S::ErfQa4, "QA4", "erf64"),
    (S::ErfQa3, "QA3", "erf64"),
    (S::ErfQa2, "QA2", "erf64"),
    (S::ErfQa1, "QA1", "erf64"),
    (S::ErfQa0, "QA0", "erf64"),
    (S::ErfRa6, "RA6", "erf64"),
    (S::ErfRa5, "RA5", "erf64"),
    (S::ErfRa4, "RA4", "erf64"),
    (S::ErfRa3, "RA3", "erf64"),
    (S::ErfRa2, "RA2", "erf64"),
    (S::ErfRa1, "RA1", "erf64"),
    (S::ErfRa0, "RA0", "erf64"),
    (S::ErfSa7, "SA7", "erf64"),
    (S::ErfSa6, "SA6", "erf64"),
    (S::ErfSa5, "SA5", "erf64"),
    (S::ErfSa4, "SA4", "erf64"),
    (S::ErfSa3, "SA3", "erf64"),
    (S::ErfSa2, "SA2", "erf64"),
    (S::ErfSa1, "SA1", "erf64"),
    (S::ErfSa0, "SA0", "erf64"),
    (S::ErfRb5, "RB5", "erf64"),
    (S::ErfRb4, "RB4", "erf64"),
    (S::ErfRb3, "RB3", "erf64"),
    (S::ErfRb2, "RB2", "erf64"),
    (S::ErfRb1, "RB1", "erf64"),
    (S::ErfRb0, "RB0", "erf64"),
    (S::ErfSb6, "SB6", "erf64"),
    (S::ErfSb5, "SB5", "erf64"),
    (S::ErfSb4, "SB4", "erf64"),
    (S::ErfSb3, "SB3", "erf64"),
    (S::ErfSb2, "SB2", "erf64"),
    (S::ErfSb1, "SB1", "erf64"),
    (S::ErfSb0, "SB0", "erf64"),
    (S::ErfPp3, "PP3", "erf64"),
    (S::ErfPp2, "PP2", "erf64"),
    (S::ErfPp1, "PP1", "erf64"),
    (S::ErfPp0, "PP0", "erf64"),
    (S::ErfQq4, "QQ4", "erf64"),
    (S::ErfQq3, "QQ3", "erf64"),
    (S::ErfQq2, "QQ2", "erf64"),
    (S::ErfQq1, "QQ1", "erf64"),
    (S::ErfQq0, "QQ0", "erf64"),
];

/// The step under test: fused when `me == fuse`, its operands recorded when `me == record`.
pub struct C {
    pub fuse: S,
    pub record: S,
    pub ops: Option<(f64, f64, f64)>,
}

impl C {
    pub fn new(fuse: S, record: S) -> Self {
        Self {
            fuse,
            record,
            ops: None,
        }
    }
    #[inline(always)]
    fn st(&mut self, me: S, c: f64, w: f64, v: f64) -> f64 {
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

// ---- src/double/pow.rs ----

const BP: [f64; 2] = [1.0, 1.5];
const DP_H: [f64; 2] = [0.0, 5.849_624_872_207_641_601_56e-01]; // 0x3fe2b803_40000000
const DP_L: [f64; 2] = [0.0, 1.350_039_202_129_748_971_28e-08]; // 0x3E4CFDEB, 0x43CFD006
const TWO53: f64 = 9_007_199_254_740_992.0; // 0x43400000_00000000
const L1: f64 = 5.999_999_999_999_946_487_25e-01; // 0x3fe33333_33333303
const L2: f64 = 4.285_714_285_785_501_842_52e-01; // 0x3fdb6db6_db6fabff
const L3: f64 = 3.333_333_298_183_774_329_18e-01; // 0x3fd55555_518f264d
const L4: f64 = 2.727_281_238_085_340_064_89e-01; // 0x3fd17460_a91d4101
const L5: f64 = 2.306_607_457_755_617_540_67e-01; // 0x3fcd864a_93c9db65
const L6: f64 = 2.069_750_178_003_384_177_84e-01; // 0x3fca7e28_4a454eef
const P1: f64 = 1.666_666_666_666_660_190_37e-01; // 0x3fc55555_5555553e
const P2: f64 = -2.777_777_777_701_559_338_42e-03; // 0xbf66c16c_16bebd93
const P3: f64 = 6.613_756_321_437_934_361_17e-05; // 0x3f11566a_af25de2c
const P4: f64 = -1.653_390_220_546_525_153_90e-06; // 0xbebbbd41_c5d26bf1
const P5: f64 = 4.138_136_797_057_238_460_39e-08; // 0x3e663769_72bea4d0
const LG2: f64 = 6.931_471_805_599_452_862_27e-01; // 0x3fe62e42_fefa39ef
const LG2_H: f64 = 6.931_471_824_645_996_093_75e-01; // 0x3fe62e43_00000000
const LG2_L: f64 = -1.904_654_299_957_768_045_25e-09; // 0xbe205c61_0ca86c39
const OVT: f64 = 8.008_566_259_537_294_437_2e-17; // -(1024-log2(ovfl+.5ulp))
const CP: f64 = 9.617_966_939_259_755_543_29e-01; // 0x3feec709_dc3a03fd =2/(3ln2)
const CP_H: f64 = 9.617_967_009_544_372_558_59e-01; // 0x3feec709_e0000000 =(float)cp
const CP_L: f64 = -7.028_461_650_952_758_265_16e-09; // 0xbe3e2fe0_145b01f5 =tail of cp_h
const IVLN2: f64 = 1.442_695_040_888_963_387_00e+00; // 0x3ff71547_652b82fe =1/ln2
const IVLN2_H: f64 = 1.442_695_021_629_333_496_09e+00; // 0x3ff71547_60000000 =24b 1/ln2
const IVLN2_L: f64 = 1.925_962_991_126_617_468_87e-08; // 0x3e54ae0b_f85ddf44 =1/ln2 tail

fn scalbn(x: f64, mut n: i32) -> f64 {
    let mut y = x;
    if n > 1023 {
        y *= f64::from_bits(0x7fe0_0000_0000_0000); // 0x1p1023
        n -= 1023;
        if n > 1023 {
            y *= f64::from_bits(0x7fe0_0000_0000_0000);
            n -= 1023;
            if n > 1023 {
                n = 1023;
            }
        }
    } else if n < -1022 {
        // make sure the final n < -53 to avoid double rounding in the
        // subnormal range: 0x1p-1022 * 0x1p53 = 0x1p-969
        y *= f64::from_bits(0x0360_0000_0000_0000);
        n += 1022 - 53;
        if n < -1022 {
            y *= f64::from_bits(0x0360_0000_0000_0000);
            n += 1022 - 53;
            if n < -1022 {
                n = -1022;
            }
        }
    }
    y * f64::from_bits(((0x3ff + n) as u64) << 52)
}

#[inline(always)]
fn high_word(x: f64) -> i32 {
    (x.to_bits() >> 32) as i32
}

#[inline(always)]
fn low_word(x: f64) -> u32 {
    x.to_bits() as u32
}

#[inline(always)]
fn with_high_word(x: f64, hi: u32) -> f64 {
    f64::from_bits((x.to_bits() & 0xffff_ffff) | (u64::from(hi) << 32))
}

#[inline(always)]
fn with_low_word(x: f64, lo: u32) -> f64 {
    f64::from_bits((x.to_bits() & 0xffff_ffff_0000_0000) | u64::from(lo))
}

/// `+inf` with sign `s` (fdlibm's `s * huge * huge`).
#[inline(always)]
fn overflow(s: f64) -> f64 {
    s * f64::INFINITY
}

/// `+0` with sign `s` (fdlibm's `s * tiny * tiny`).
#[inline(always)]
fn underflow(s: f64) -> f64 {
    s * 0.0
}

/// fdlibm `__ieee754_pow(x, y)`.
pub fn pow(x: f64, y: f64, cx: &mut C) -> f64 {
    let (hx, lx) = (high_word(x), low_word(x));
    let (hy, ly) = (high_word(y), low_word(y));
    let mut ix = hx & 0x7fff_ffff;
    let iy = hy & 0x7fff_ffff;

    // y == ±0: x**0 = 1, even if x is NaN
    if (iy as u32 | ly) == 0 {
        return 1.0;
    }
    // x == +1: 1**y = 1, even if y is NaN
    if hx == 0x3ff0_0000 && lx == 0 {
        return 1.0;
    }
    // NaN if either argument is NaN (the canonical one)
    if ix > 0x7ff0_0000
        || (ix == 0x7ff0_0000 && lx != 0)
        || iy > 0x7ff0_0000
        || (iy == 0x7ff0_0000 && ly != 0)
    {
        return f64::NAN;
    }

    // yisint = 0: y is not an integer, 1: an odd integer, 2: an even integer
    let mut yisint = 0i32;
    if hx < 0 {
        if iy >= 0x4340_0000 {
            yisint = 2; // |y| >= 2^53: an even integer
        } else if iy >= 0x3ff0_0000 {
            let k = (iy >> 20) - 0x3ff; // exponent
            if k > 20 {
                let j = ly >> (52 - k);
                if (j << (52 - k)) == ly {
                    yisint = 2 - (j & 1) as i32;
                }
            } else if ly == 0 {
                let j = iy >> (20 - k);
                if (j << (20 - k)) == iy {
                    yisint = 2 - (j & 1);
                }
            }
        }
    }

    // special values of y
    if ly == 0 {
        if iy == 0x7ff0_0000 {
            // y is ±inf
            return if ((ix - 0x3ff0_0000) | lx as i32) == 0 {
                1.0 // (-1)**±inf is 1
            } else if ix >= 0x3ff0_0000 {
                // (|x| > 1)**±inf = inf, 0
                if hy >= 0 {
                    y
                } else {
                    0.0
                }
            } else {
                // (|x| < 1)**±inf = 0, inf
                if hy >= 0 {
                    0.0
                } else {
                    -y
                }
            };
        }
        if iy == 0x3ff0_0000 {
            // y is ±1
            return if hy >= 0 { x } else { 1.0 / x };
        }
        if hy == 0x4000_0000 {
            return x * x; // y is 2
        }
        if hy == 0x3fe0_0000 && hx >= 0 {
            return dm::sqrt64(x); // y is 0.5 and x >= +0
        }
    }

    let mut ax = x.abs();
    // special values of x: ±0, ±inf, ±1
    if lx == 0 && (ix == 0x7ff0_0000 || ix == 0 || ix == 0x3ff0_0000) {
        let mut z = ax;
        if hy < 0 {
            z = 1.0 / z; // z = 1/|x|
        }
        if hx < 0 {
            if ((ix - 0x3ff0_0000) | yisint) == 0 {
                return f64::NAN; // (-1)**non-int is NaN
            } else if yisint == 1 {
                z = -z; // (x<0)**odd = -(|x|**odd)
            }
        }
        return z;
    }

    let mut s = 1.0; // sign of the result
    if hx < 0 {
        if yisint == 0 {
            return f64::NAN; // (x<0)**(non-int) is NaN
        }
        if yisint == 1 {
            s = -1.0; // (x<0)**(odd int)
        }
    }

    let (t1, t2);
    if iy > 0x41e0_0000 {
        // |y| > 2^31
        if iy > 0x43f0_0000 {
            // |y| > 2^64: must over/underflow
            if ix <= 0x3fef_ffff {
                return if hy < 0 { f64::INFINITY } else { 0.0 };
            }
            if ix >= 0x3ff0_0000 {
                return if hy > 0 { f64::INFINITY } else { 0.0 };
            }
        }
        // over/underflow if x is not close to one
        if ix < 0x3fef_ffff {
            return if hy < 0 { overflow(s) } else { underflow(s) };
        }
        if ix > 0x3ff0_0000 {
            return if hy > 0 { overflow(s) } else { underflow(s) };
        }
        // now |1-x| is tiny <= 2^-20, enough to compute log(x) by x-x^2/2+x^3/3-x^4/4
        let t = ax - 1.0; // t has 20 trailing zeros
        let w = (t * t) * (0.5 - t * (0.333_333_333_333_333_333_333_3 - t * 0.25));
        let u = IVLN2_H * t; // ivln2_h has 21 significant bits
        let v = t * IVLN2_L - w * IVLN2;
        t1 = with_low_word(u + v, 0);
        t2 = v - (t1 - u);
    } else {
        let mut n = 0i32;
        // take care of subnormal x
        if ix < 0x0010_0000 {
            ax *= TWO53;
            n -= 53;
            ix = high_word(ax);
        }
        n += (ix >> 20) - 0x3ff;
        let j = ix & 0x000f_ffff;
        // determine the interval
        ix = j | 0x3ff0_0000; // normalize ix
        let k: usize = if j <= 0x3988e {
            0 // |x| < sqrt(3/2)
        } else if j < 0xbb67a {
            1 // |x| < sqrt(3)
        } else {
            n += 1;
            ix -= 0x0010_0000;
            0
        };
        ax = with_high_word(ax, ix as u32);

        // ss = s_h + s_l = (x-1)/(x+1) or (x-1.5)/(x+1.5)
        let u = ax - BP[k];
        let v = 1.0 / (ax + BP[k]);
        let ss = u * v;
        let s_h = with_low_word(ss, 0);
        // t_h = ax + bp[k], high part
        let t_h = with_high_word(
            0.0,
            ((ix as u32 >> 1) | 0x2000_0000) + 0x0008_0000 + ((k as u32) << 18),
        );
        let t_l = ax - (t_h - BP[k]);
        let s_l = v * ((u - s_h * t_h) - s_h * t_l);
        // log(ax)
        let s2 = ss * ss;
        let l5 = cx.st(S::PowL5, L5, s2, L6);
        let l4 = cx.st(S::PowL4, L4, s2, l5);
        let l3 = cx.st(S::PowL3, L3, s2, l4);
        let l2 = cx.st(S::PowL2, L2, s2, l3);
        let l1 = cx.st(S::PowL1, L1, s2, l2);
        let mut r = s2 * s2 * l1;
        r += s_l * (s_h + ss);
        let s2 = s_h * s_h;
        let t_h = with_low_word(3.0 + s2 + r, 0);
        let t_l = r - ((t_h - 3.0) - s2);
        // u + v = ss*(1+...)
        let u = s_h * t_h;
        let v = s_l * t_h + t_l * ss;
        // 2/(3log2)*(ss+...)
        let p_h = with_low_word(u + v, 0);
        let p_l = v - (p_h - u);
        let z_h = CP_H * p_h; // cp_h + cp_l = 2/(3*log2)
        let z_l = CP_L * p_h + p_l * CP + DP_L[k];
        // log2(ax) = (ss+..)*2/(3*log2) = n + dp_h + z_h + z_l
        let t = f64::from(n);
        t1 = with_low_word(((z_h + z_l) + DP_H[k]) + t, 0);
        t2 = z_l - (((t1 - t) - DP_H[k]) - z_h);
    }

    // split y into y1 + y2 and compute (y1+y2)*(t1+t2)
    let y1 = with_low_word(y, 0);
    let p_l = (y - y1) * t1 + y * t2;
    let mut p_h = y1 * t1;
    let z = p_l + p_h;
    let mut j = high_word(z);
    let i = low_word(z) as i32;
    if j >= 0x4090_0000 {
        // z >= 1024
        if ((j - 0x4090_0000) | i) != 0 {
            return overflow(s); // z > 1024
        }
        if p_l + OVT > z - p_h {
            return overflow(s);
        }
    } else if (j & 0x7fff_ffff) >= 0x4090_cc00 {
        // z <= -1075
        if ((j as u32).wrapping_sub(0xc090_cc00) | i as u32) != 0 {
            return underflow(s); // z < -1075
        }
        if p_l <= z - p_h {
            return underflow(s);
        }
    }

    // 2**(p_h + p_l)
    let i = j & 0x7fff_ffff;
    let mut k = (i >> 20) - 0x3ff;
    let mut n = 0i32;
    if i > 0x3fe0_0000 {
        // |z| > 0.5: n = [z + 0.5]
        n = j + (0x0010_0000 >> (k + 1));
        k = ((n & 0x7fff_ffff) >> 20) - 0x3ff; // new k for n
        let t = with_high_word(0.0, (n & !(0x000f_ffff >> k)) as u32);
        n = ((n & 0x000f_ffff) | 0x0010_0000) >> (20 - k);
        if j < 0 {
            n = -n;
        }
        p_h -= t;
    }
    let t = with_low_word(p_l + p_h, 0);
    let u = t * LG2_H;
    let v = (p_l - (t - p_h)) * LG2 + t * LG2_L;
    let mut z = u + v;
    let w = v - (z - u);
    let t = z * z;
    let p4 = cx.st(S::PowP4, P4, t, P5);
    let p3 = cx.st(S::PowP3, P3, t, p4);
    let p2 = cx.st(S::PowP2, P2, t, p3);
    let p1 = cx.st(S::PowP1, P1, t, p2);
    let t1 = z - t * p1;
    let r = (z * t1) / (t1 - 2.0) - (w + z * w);
    z = 1.0 - (r - z);
    j = high_word(z);
    j += n << 20;
    if (j >> 20) <= 0 {
        z = scalbn(z, n); // subnormal output
    } else {
        z = with_high_word(z, j as u32);
    }
    s * z
}

// ---- src/erf.rs ----

const ERX: f64 = 8.450_629_115_104_675_292_97e-01;
const EFX8: f64 = 1.027_033_336_764_100_690_53e+00;
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
const ERFC_SWITCH: f64 = 2.857_141_494_750_976_6;
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
fn erfc_mid(ax: f64, cx: &mut C) -> f64 {
    if ax < 1.25 {
        let s = ax - 1.0;
        let pa5 = cx.st(S::ErfPa5, PA5, s, PA6);
        let pa4 = cx.st(S::ErfPa4, PA4, s, pa5);
        let pa3 = cx.st(S::ErfPa3, PA3, s, pa4);
        let pa2 = cx.st(S::ErfPa2, PA2, s, pa3);
        let pa1 = cx.st(S::ErfPa1, PA1, s, pa2);
        let pa0 = cx.st(S::ErfPa0, PA0, s, pa1);
        let qa5 = cx.st(S::ErfQa5, QA5, s, QA6);
        let qa4 = cx.st(S::ErfQa4, QA4, s, qa5);
        let qa3 = cx.st(S::ErfQa3, QA3, s, qa4);
        let qa2 = cx.st(S::ErfQa2, QA2, s, qa3);
        let qa1 = cx.st(S::ErfQa1, QA1, s, qa2);
        let qa0 = cx.st(S::ErfQa0, 1.0, s, qa1);
        let (p, q) = (pa0, qa0);
        return 1.0 - ERX - p / q;
    }
    let s = 1.0 / (ax * ax);
    let (r, big_s) = if ax < ERFC_SWITCH {
        let ra6 = cx.st(S::ErfRa6, RA6, s, RA7);
        let ra5 = cx.st(S::ErfRa5, RA5, s, ra6);
        let ra4 = cx.st(S::ErfRa4, RA4, s, ra5);
        let ra3 = cx.st(S::ErfRa3, RA3, s, ra4);
        let ra2 = cx.st(S::ErfRa2, RA2, s, ra3);
        let ra1 = cx.st(S::ErfRa1, RA1, s, ra2);
        let ra0 = cx.st(S::ErfRa0, RA0, s, ra1);
        let sa7 = cx.st(S::ErfSa7, SA7, s, SA8);
        let sa6 = cx.st(S::ErfSa6, SA6, s, sa7);
        let sa5 = cx.st(S::ErfSa5, SA5, s, sa6);
        let sa4 = cx.st(S::ErfSa4, SA4, s, sa5);
        let sa3 = cx.st(S::ErfSa3, SA3, s, sa4);
        let sa2 = cx.st(S::ErfSa2, SA2, s, sa3);
        let sa1 = cx.st(S::ErfSa1, SA1, s, sa2);
        let sa0 = cx.st(S::ErfSa0, 1.0, s, sa1);
        (ra0, sa0)
    } else {
        let rb5 = cx.st(S::ErfRb5, RB5, s, RB6);
        let rb4 = cx.st(S::ErfRb4, RB4, s, rb5);
        let rb3 = cx.st(S::ErfRb3, RB3, s, rb4);
        let rb2 = cx.st(S::ErfRb2, RB2, s, rb3);
        let rb1 = cx.st(S::ErfRb1, RB1, s, rb2);
        let rb0 = cx.st(S::ErfRb0, RB0, s, rb1);
        let sb6 = cx.st(S::ErfSb6, SB6, s, SB7);
        let sb5 = cx.st(S::ErfSb5, SB5, s, sb6);
        let sb4 = cx.st(S::ErfSb4, SB4, s, sb5);
        let sb3 = cx.st(S::ErfSb3, SB3, s, sb4);
        let sb2 = cx.st(S::ErfSb2, SB2, s, sb3);
        let sb1 = cx.st(S::ErfSb1, SB1, s, sb2);
        let sb0 = cx.st(S::ErfSb0, 1.0, s, sb1);
        (rb0, sb0)
    };
    // -x² = -z² + (z - x)(z + x) with z = x truncated to its high 32 bits, so
    // z² is exact and the large part of the exponent carries no rounding.
    let z = f64::from_bits(ax.to_bits() & 0xffff_ffff_0000_0000);
    dm::exp64(-z * z - 0.5625) * dm::exp64((z - ax) * (z + ax) + r / big_s) / ax
}

/// Deterministic error function `erf(x) = 2/√π ∫₀ˣ e^{-t²} dt` in double
/// precision (musl `erf.c`, from fdlibm `s_erf.c`, with [`exp64`] for the
/// exponentials).
///
/// `erf64(±inf) = ±1`, `erf64(NaN) = NaN`, `erf64(-0) = -0`; `|x| ≥ 6` returns
/// `±1` (the true value is within `2^-53` of it from `|x| ≈ 5.92`).
pub fn erf64(x: f64, cx: &mut C) -> f64 {
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
        let pp3 = cx.st(S::ErfPp3, PP3, z, PP4);
        let pp2 = cx.st(S::ErfPp2, PP2, z, pp3);
        let pp1 = cx.st(S::ErfPp1, PP1, z, pp2);
        let pp0 = cx.st(S::ErfPp0, PP0, z, pp1);
        let qq4 = cx.st(S::ErfQq4, QQ4, z, QQ5);
        let qq3 = cx.st(S::ErfQq3, QQ3, z, qq4);
        let qq2 = cx.st(S::ErfQq2, QQ2, z, qq3);
        let qq1 = cx.st(S::ErfQq1, QQ1, z, qq2);
        let qq0 = cx.st(S::ErfQq0, 1.0, z, qq1);
        let (r, s) = (pp0, qq0);
        return x + x * (r / s);
    }
    let y = if ax < 6.0 {
        1.0 - erfc_mid(ax, cx)
    } else {
        1.0
    };
    if x < 0.0 {
        -y
    } else {
        y
    }
}

// ---- inputs: the band where the step's variable is largest ----

fn splitmix64(mut z: u64) -> u64 {
    z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    z ^ (z >> 31)
}

fn unit(u: u64) -> f64 {
    (u >> 11) as f64 * (1.0 / 9_007_199_254_740_992.0)
}

fn lerp(a: f64, b: f64, u: f64) -> f64 {
    a + (b - a) * u
}

/// The input pair `(x, y)` of index `i` for step `s` (`y` is unused by `erf64`).
///
/// * `erf64`: `|x|` in the upper part of the branch whose polynomial holds the step (`z = x²`
///   up to 0.71 for `PP` / `QQ`; `s = |x| - 1` near both ends of `[-0.156, 0.25)` for `PA` /
///   `QA`; `s = 1/x²` near 0.64 for `RA` / `SA`, near 0.12 for `RB` / `SB`).
/// * `powf64` `L*`: `x = m` with the mantissa near the top of a log interval (`|ss|` up to
///   0.1), `y` so that `|y·log2 x|` is 100 to 1000 (a large exponent amplifies the log's error).
/// * `powf64` `P*`: `y·log2 x = n + f` with `|f|` in `[0.4, 0.5]`, so the reduced `z` of the
///   exponential is near its largest `0.5·ln 2`.
pub fn input(s: S, seed: u64, i: u64) -> (f64, f64) {
    let h = splitmix64(seed ^ splitmix64(i));
    let pick = h & 0xf;
    let sign = if (h >> 4) & 1 == 1 { -1.0 } else { 1.0 };
    let u = unit(splitmix64(h));
    let u2 = unit(splitmix64(h ^ 0x5bd1_e995));
    let name = format!("{s:?}");
    if let Some(rest) = name.strip_prefix("Erf") {
        let ax = match &rest[..2] {
            "Pp" | "Qq" => lerp(0.6, 0.843_75, u),
            "Pa" | "Qa" => {
                if pick & 1 == 0 {
                    lerp(1.15, 1.25, u)
                } else {
                    lerp(0.843_75, 0.9, u)
                }
            }
            "Ra" | "Sa" => lerp(1.25, 1.6, u),
            _ => lerp(ERFC_SWITCH, 3.6, u),
        };
        return (ax * sign, 0.0);
    }
    let x = match pick % 3 {
        0 => lerp(1.18, 1.224_7, u),
        1 => lerp(1.65, 1.732, u),
        _ => lerp(1.732, 1.999, u),
    };
    let lg = dm::log2_64(x);
    let target = if name.starts_with("PowL") {
        lerp(100.0, 1000.0, u2) * sign
    } else {
        let n = ((h >> 8) % 2000) as f64 - 1000.0;
        n + lerp(0.4, 0.5, u2) * sign
    };
    (x, target / lg)
}

pub fn eval(f: &str, x: f64, y: f64, cx: &mut C) -> f64 {
    match f {
        "powf64" => pow(x, y, cx),
        "erf64" => erf64(x, cx),
        _ => unreachable!(),
    }
}

pub fn library(f: &str, x: f64, y: f64) -> f64 {
    match f {
        "powf64" => dm::powf64(x, y),
        "erf64" => dm::erf64(x),
        _ => unreachable!(),
    }
}

pub struct Found {
    pub index: u64,
    pub x: f64,
    pub y: f64,
    pub out: f64,
    pub fused: f64,
}

#[derive(Default)]
pub struct Tally {
    pub compared: u64,
    pub flips: u64,
    pub changed: u64,
    pub library_checked: u64,
    pub found: Vec<Found>,
    pub drift: bool,
}

/// The `powf64` log steps, searched over `x` with many `y` per `x` (see [`search`]).
const FAN_OUT: [S; 5] = [S::PowL1, S::PowL2, S::PowL3, S::PowL4, S::PowL5];

/// Draws `lo..hi` for step `s`: the error-free test at the step, then (where the step's two
/// roundings differ from one) both builds in full; every 4096th input and every one that
/// reached the full evaluation is also compared with the library.
pub fn search(s: S, f: &str, seed: u64, lo: u64, hi: u64, keep: usize) -> Tally {
    let mut t = Tally::default();
    for i in lo..hi {
        let (x, y) = input(s, seed, i);
        t.compared += 1;
        let mut cx = C::new(S::None, s);
        let out = eval(f, x, y, &mut cx);
        let flips = cx
            .ops
            .is_some_and(|(c, w, v)| w.mul_add(v, c).to_bits() != (c + w * v).to_bits());
        if !flips && i % 4096 != 0 {
            continue;
        }
        if out.to_bits() != library(f, x, y).to_bits() {
            eprintln!(
                "copy drift: {f}({:#018x}, {:#018x}) = {:#018x}, library {:#018x}",
                x.to_bits(),
                y.to_bits(),
                out.to_bits(),
                library(f, x, y).to_bits()
            );
            t.drift = true;
            return t;
        }
        t.library_checked += 1;
        if !flips {
            continue;
        }
        t.flips += 1;
        if FAN_OUT.contains(&s) {
            // the log steps depend on x alone: where one flips, its effect on log2 x reaches the
            // output scaled by y, so try many y with |y·log2 x| near the overflow edge
            let lg = dm::log2_64(x);
            for j in 0..256u64 {
                let u = unit(splitmix64(seed ^ i.rotate_left(17) ^ j));
                let sign = if j & 1 == 1 { -1.0 } else { 1.0 };
                let yj = lerp(900.0, 1020.0, u) * sign / lg;
                let a = eval(f, x, yj, &mut C::new(S::None, S::None));
                let b = eval(f, x, yj, &mut C::new(s, S::None));
                if a.to_bits() != b.to_bits() {
                    if a.to_bits() != library(f, x, yj).to_bits() {
                        t.drift = true;
                        return t;
                    }
                    t.changed += 1;
                    if t.found.len() < keep {
                        t.found.extend([Found {
                            index: i,
                            x,
                            y: yj,
                            out: a,
                            fused: b,
                        }]);
                    }
                    break;
                }
            }
            continue;
        }
        let fused = eval(f, x, y, &mut C::new(s, S::None));
        if fused.to_bits() != out.to_bits() {
            t.changed += 1;
            if t.found.len() < keep {
                t.found.extend([Found {
                    index: i,
                    x,
                    y,
                    out,
                    fused,
                }]);
            }
        }
    }
    t
}

/// The modelled step of `(function, pin name)`, if any.
pub fn step_of(f: &str, name: &str) -> Option<S> {
    STEPS
        .iter()
        .find(|(_, n, g)| *n == name && *g == f)
        .map(|(s, _, _)| *s)
}

/// Whether a pin separates its step: the library gives `want` and the build with the step
/// fused does not.
pub fn pin_separates(f: &str, s: S, x: f64, y: f64, want: u64) -> (u64, u64, bool) {
    let lib = library(f, x, y).to_bits();
    let fused = eval(f, x, y, &mut C::new(s, S::None)).to_bits();
    (lib, fused, lib == want && fused != want)
}

/// The modelled steps of `powf64` / `erf64` that no pin in `text` separates, as `function:NAME`.
/// `--check-pins` fails while this is not empty: the claim that every polynomial step of these
/// kernels is separated by a pin holds only when it is.
pub fn steps_without_pin(text: &str) -> Vec<String> {
    let flat: String = text.chars().filter(|c| !c.is_whitespace()).collect();
    STEPS
        .iter()
        .filter(|(_, n, f)| !flat.contains(&format!("(\"{f}\",\"{n}\",0x")))
        .map(|(_, n, f)| format!("{f}:{n}"))
        .collect()
}

// ---- enumeration of the step's variable (`--pow-erf-enumerate STEP LO HI`) ----
//
// For the steps the draws do not reach, every double `v` of the step's variable with bits in
// [LO, HI) is walked (for a step `c + v·k` with constants `c`, `k`, only the values whose step can
// round differently, by `crate::window_ck`; otherwise every value), followed while the fused step
// changes the part of the polynomial that depends on `v` alone, and then turned into inputs:
//
// * `powf64` `L*` (`v = s2 = fl(ss·ss)`): `ss` from `±√v`, `x` from `ss = (x - b)/(x + b)` in each
//   of e_pow's three intervals, and for each `x` 256 exponents `y` with `|y·log2 x|` in
//   [900, 1020] (the log's change reaches the output scaled by `y`);
// * `powf64` `P4` (`v = t = fl(z·z)`): `z` from `±√v`, and `y` with `pow(2, y)` reducing to that
//   `z` (`log2 2 = 1` exactly, so the exponential's argument is `y` itself for `|y| ≤ 0.5`);
// * `erf64` `SB*` (`v = s = fl(1/x²)`): `x` from `1/√v`.
//
// Each preimage is kept only if the forward computation gives `v` back, so a reported input
// reaches the step with that `v`; each is evaluated in full, both builds and the library.

/// The doubles within `span` ulps of `y0` (both directions) for which `fwd` gives `v` back.
fn preimages(y0: f64, span: i64, v: f64, fwd: impl Fn(f64) -> f64, out: &mut Vec<f64>) {
    for d in -span..=span {
        let y = f64::from_bits(y0.to_bits().wrapping_add_signed(d));
        if y.is_finite() && fwd(y).to_bits() == v.to_bits() && !out.contains(&y) {
            out.extend([y]);
        }
    }
}

/// e_pow's `ss` for `x` in [1, 2) (the normalisation of `|x|` and the choice of `bp`).
fn pow_ss(x: f64) -> f64 {
    let ix = high_word(x) & 0x7fff_ffff;
    let j = ix & 0x000f_ffff;
    let (k, ix) = if j <= 0x3988e {
        (0usize, j | 0x3ff0_0000)
    } else if j < 0xbb67a {
        (1, j | 0x3ff0_0000)
    } else {
        (0, (j | 0x3ff0_0000) - 0x0010_0000)
    };
    let ax = with_high_word(x, ix as u32);
    (ax - BP[k]) * (1.0 / (ax + BP[k]))
}

/// e_pow's reduced argument `z` of the exponential for `pow(2, y)`, `|y| ≤ 0.5`.
fn pow2_z(y: f64) -> f64 {
    let y1 = with_low_word(y, 0);
    let p_l = y - y1;
    let p_h = y1;
    let t = with_low_word(p_l + p_h, 0);
    let u = t * LG2_H;
    let v = (p_l - (t - p_h)) * LG2 + t * LG2_L;
    u + v
}

/// `(c, k)` of a step `c + v·k` with both constant, for the window solver.
fn constant_step(s: S) -> Option<(f64, f64)> {
    match s {
        S::PowL5 => Some((L5, L6)),
        S::PowP4 => Some((P4, P5)),
        S::ErfSb6 => Some((SB6, SB7)),
        _ => None,
    }
}

/// The steps `--pow-erf-enumerate` models.
pub const WALKED: [S; 5] = [S::PowL5, S::PowL4, S::PowL3, S::PowP4, S::ErfSb6];

/// The step's own rounding and the part of the polynomial that depends on `v` alone, with the
/// step fused or not: `(step value, part)` bits.
fn step_and_part(s: S, v: f64, fuse: S) -> (u64, u64) {
    let mut cx = C::new(fuse, s);
    let part = match s {
        S::PowL5 | S::PowL4 | S::PowL3 => {
            let l5 = cx.st(S::PowL5, L5, v, L6);
            let l4 = cx.st(S::PowL4, L4, v, l5);
            let l3 = cx.st(S::PowL3, L3, v, l4);
            let l2 = cx.st(S::PowL2, L2, v, l3);
            let l1 = cx.st(S::PowL1, L1, v, l2);
            v * v * l1
        }
        S::PowP4 => {
            let p4 = cx.st(S::PowP4, P4, v, P5);
            let p3 = cx.st(S::PowP3, P3, v, p4);
            let p2 = cx.st(S::PowP2, P2, v, p3);
            let p1 = cx.st(S::PowP1, P1, v, p2);
            v * p1
        }
        S::ErfSb6 => {
            let sb6 = cx.st(S::ErfSb6, SB6, v, SB7);
            let sb5 = cx.st(S::ErfSb5, SB5, v, sb6);
            let sb4 = cx.st(S::ErfSb4, SB4, v, sb5);
            let sb3 = cx.st(S::ErfSb3, SB3, v, sb4);
            let sb2 = cx.st(S::ErfSb2, SB2, v, sb3);
            let sb1 = cx.st(S::ErfSb1, SB1, v, sb2);
            cx.st(S::ErfSb0, 1.0, v, sb1)
        }
        _ => unreachable!(),
    };
    let (c, w, k) = cx.ops.expect("the step is on this path");
    let step = if fuse == s {
        w.mul_add(k, c)
    } else {
        c + w * k
    };
    (step.to_bits(), part.to_bits())
}

/// The inputs `(x, y)` that reach step `s` with variable `v`, into `out`.
fn inputs_of(s: S, v: f64, seed: u64, out: &mut Vec<(f64, f64)>) {
    out.clear();
    let mut a = Vec::new();
    match s {
        S::PowL5 | S::PowL4 | S::PowL3 => {
            let mut sss = Vec::new();
            let r = dm::sqrt64(v);
            preimages(r, 4, v, |q| q * q, &mut sss);
            preimages(-r, 4, v, |q| q * q, &mut sss);
            for &ss in &sss {
                let c0 = (1.0 + ss) / (1.0 - ss);
                for x0 in [c0, 1.5 * c0, 2.0 * c0] {
                    if (1.0..2.0).contains(&x0) {
                        preimages(x0, 8, ss, pow_ss, &mut a);
                    }
                }
            }
            for &x in &a {
                let lg = dm::log2_64(x);
                for j in 0..256u64 {
                    let u = unit(splitmix64(seed ^ x.to_bits().rotate_left(13) ^ j));
                    let sign = if j & 1 == 1 { -1.0 } else { 1.0 };
                    out.extend([(x, lerp(900.0, 1020.0, u) * sign / lg)]);
                }
            }
        }
        S::PowP4 => {
            let mut zs = Vec::new();
            let r = dm::sqrt64(v);
            preimages(r, 4, v, |q| q * q, &mut zs);
            preimages(-r, 4, v, |q| q * q, &mut zs);
            for &z in &zs {
                let y0 = z * core::f64::consts::LOG2_E;
                preimages(y0, 64, z, pow2_z, &mut a);
            }
            for &y in &a {
                if y.abs() <= 0.5 {
                    out.extend([(2.0, y)]);
                }
            }
            // other bases: `y ≈ z / (ln 2 · log2 x)`, kept where the copy's step reads `v`
            // itself (the recorded operand), so the input reaches the step with this `v`
            for j in 0..64u64 {
                let x = lerp(1.05, 1.95, unit(splitmix64(seed ^ (j << 32) ^ 0x7034)));
                let lg = dm::log2_64(x);
                for &z in &zs {
                    let y0 = z * core::f64::consts::LOG2_E / lg;
                    for d in -8i64..=8 {
                        let y = f64::from_bits(y0.to_bits().wrapping_add_signed(d));
                        let mut cx = C::new(S::None, S::PowP4);
                        let _ = pow(x, y, &mut cx);
                        if cx.ops.is_some_and(|(_, w, _)| w.to_bits() == v.to_bits()) {
                            out.extend([(x, y)]);
                        }
                    }
                }
            }
        }
        S::ErfSb6 => {
            let x0 = 1.0 / dm::sqrt64(v);
            preimages(x0, 8, v, |x| 1.0 / (x * x), &mut a);
            for &x in &a {
                if (ERFC_SWITCH..6.0).contains(&x) {
                    out.extend([(x, 0.0), (-x, 0.0)]);
                }
            }
        }
        _ => unreachable!(),
    }
}

/// Walks the variable of step `s` over bits [lo, hi) (see the comment above).
pub fn walk(s: S, f: &str, lo: u64, hi: u64, keep: usize) -> Tally {
    let mut t = Tally::default();
    let mut xs = Vec::new();
    let mut visit = |bits: u64, t: &mut Tally| -> bool {
        let v = f64::from_bits(bits);
        t.compared += 1;
        let plain = step_and_part(s, v, S::None);
        let fused = step_and_part(s, v, s);
        if plain.0 == fused.0 {
            return true;
        }
        t.flips += 1;
        if plain.1 == fused.1 {
            return true;
        }
        inputs_of(s, v, bits, &mut xs);
        for &(x, y) in &xs {
            let out = eval(f, x, y, &mut C::new(S::None, S::None));
            if out.to_bits() != library(f, x, y).to_bits() {
                eprintln!(
                    "copy drift: {f}({:#018x}, {:#018x})",
                    x.to_bits(),
                    y.to_bits()
                );
                t.drift = true;
                return false;
            }
            t.library_checked += 1;
            let fz = eval(f, x, y, &mut C::new(s, S::None));
            if fz.to_bits() != out.to_bits() {
                t.changed += 1;
                if t.found.len() < keep {
                    t.found.extend([Found {
                        index: bits,
                        x,
                        y,
                        out,
                        fused: fz,
                    }]);
                }
                break;
            }
        }
        true
    };
    if let Some((c, k)) = constant_step(s) {
        crate::window_ck(c, k, lo, hi, |bits| visit(bits, &mut t));
    } else {
        for bits in lo..hi {
            if !visit(bits, &mut t) {
                break;
            }
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    // pins of tests/contraction_goldens.rs (tools/contraction-search --pow-erf)
    const ERF_PP3: (u64, u64) = (0x3fea_d205_af57_af16, 0x3fe8_7388_b64e_cb54);
    const POW_L1: (u64, u64, u64) = (
        0x3ff3_70d9_6ab8_e0c5,
        0xc090_a7e0_bc8f_47f8,
        0x2d35_c5ea_eb55_f15c,
    );

    #[test]
    fn the_copies_are_the_library_with_no_step_fused() {
        for i in 0..4096 {
            for &(s, _, f) in STEPS {
                let (x, y) = input(s, 0x1234, i);
                let copy = eval(f, x, y, &mut C::new(S::None, S::None)).to_bits();
                assert_eq!(copy, library(f, x, y).to_bits(), "{f}({x:e}, {y:e})");
            }
        }
    }

    #[test]
    fn an_erf_pin_separates_its_step_and_only_with_its_bits() {
        let s = step_of("erf64", "PP3").expect("modelled");
        let x = f64::from_bits(ERF_PP3.0);
        assert!(pin_separates("erf64", s, x, 0.0, ERF_PP3.1).2);
        // the bits of the fused build, or one ulp off, are not the library's
        let (_, fused, _) = pin_separates("erf64", s, x, 0.0, ERF_PP3.1);
        assert!(!pin_separates("erf64", s, x, 0.0, fused).2);
        assert!(!pin_separates("erf64", s, x, 0.0, ERF_PP3.1 + 1).2);
        // another step fused at this input leaves the output: the pin names its own step
        let other = step_of("erf64", "PP0").expect("modelled");
        assert!(!pin_separates("erf64", other, x, 0.0, ERF_PP3.1).2 || other == s);
    }

    #[test]
    fn a_pow_pin_reads_both_arguments() {
        let s = step_of("powf64", "L1").expect("modelled");
        let (x, y) = (f64::from_bits(POW_L1.0), f64::from_bits(POW_L1.1));
        assert!(pin_separates("powf64", s, x, y, POW_L1.2).2);
        assert!(!pin_separates("powf64", s, x, -y, POW_L1.2).2);
    }

    #[test]
    fn a_step_without_a_pin_is_reported() {
        let one = format!(
            "(\"erf64\", \"PP3\", {:#018x}, {:#018x}),",
            ERF_PP3.0, ERF_PP3.1
        );
        let missing = steps_without_pin(&one);
        assert_eq!(missing.len(), STEPS.len() - 1);
        assert!(!missing.iter().any(|m| m == "erf64:PP3"));
        assert!(missing.iter().any(|m| m == "powf64:L1"));
    }
}
