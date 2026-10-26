//! `pow` in double precision over every pair of `f64` arguments.
//!
//! A line-by-line port of fdlibm 5.3 `e_pow.c` as kept in FreeBSD msun
//! (`log2(x)` as a two-word value with 29 trailing zero bits in the high word,
//! `y·log2(x) = n + y'` in simulated extra precision, `2^n · exp(y'·ln 2)`).
//! Every operation is an IEEE 754 basic operation in the source's order, and
//! the special cases are those of IEEE 754 `pow` (C99 Annex F.10.4.4). Only
//! two things differ from the C source:
//!
//! * a NaN result is the canonical NaN instead of `x + y` / `(x - x) / (x - x)`,
//!   whose sign and payload are the platform's;
//! * `huge * huge` / `tiny * tiny`, which only exist to raise the overflow /
//!   underflow flags, are the infinity / zero they evaluate to (Rust does not
//!   observe the flags).
//!
//! ====================================================
//! Copyright (C) 2004 by Sun Microsystems, Inc. All rights reserved.
//!
//! Permission to use, copy, modify, and distribute this
//! software is freely granted, provided that this notice
//! is preserved.
//! ====================================================

use super::trig::scalbn;
use crate::ops::sqrt64;

const BP: [f64; 2] = [1.0, 1.5];
const DP_H: [f64; 2] = [0.0, 5.849_624_872_207_641_601_56e-01]; // 0x3fe2b803_40000000
const DP_L: [f64; 2] = [0.0, 1.350_039_202_129_748_971_28e-08]; // 0x3E4CFDEB, 0x43CFD006
const TWO53: f64 = 9_007_199_254_740_992.0; // 0x43400000_00000000

// poly coefs for (3/2)*(log(x)-2s-2/3*s**3
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
#[allow(
    clippy::too_many_lines,
    clippy::cast_sign_loss,
    clippy::cast_possible_wrap
)]
pub(super) fn pow(x: f64, y: f64) -> f64 {
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
            return sqrt64(x); // y is 0.5 and x >= +0
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
        let mut r = s2 * s2 * (L1 + s2 * (L2 + s2 * (L3 + s2 * (L4 + s2 * (L5 + s2 * L6)))));
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
    let t1 = z - t * (P1 + t * (P2 + t * (P3 + t * (P4 + t * P5))));
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
