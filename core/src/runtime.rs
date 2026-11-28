//! The runtime group: the bit-exact runtime calls of the game logic.
#![allow(clippy::all)]

use crate::Call;

include!("runtime_tables.rs");

// ---------------------------------------------------------------- argument helpers

struct A(Vec<String>);

impl A {
    fn parse(text: &str) -> A {
        let t = text.trim();
        let t = t.strip_prefix('[').unwrap_or(t);
        let t = t.strip_suffix(']').unwrap_or(t);
        A(t.split(',').map(|s| s.trim().trim_matches('"').to_string()).collect())
    }
    fn raw(&self, k: usize) -> &str {
        self.0.get(k).map(|s| s.as_str()).unwrap_or("")
    }
    fn i(&self, k: usize) -> i32 {
        let s = self.raw(k);
        s.parse::<i64>().map(|v| v as i32).or_else(|_| s.parse::<u64>().map(|v| v as i32)).unwrap_or(0)
    }
    fn u(&self, k: usize) -> u32 {
        self.i(k) as u32
    }
    fn f(&self, k: usize) -> u64 {
        self.raw(k).parse::<u64>().unwrap_or(0)
    }
    fn b(&self, k: usize) -> Vec<u8> {
        unhex(self.raw(k))
    }
    /// A C string: the bytes up to the first 0.
    fn cs(&self, k: usize) -> Vec<u8> {
        let mut v = self.b(k);
        if let Some(p) = v.iter().position(|&c| c == 0) {
            v.truncate(p);
        }
        v
    }
}

fn unhex(s: &str) -> Vec<u8> {
    let b = s.as_bytes();
    let h = |c: u8| match c {
        b'0'..=b'9' => c - b'0',
        b'a'..=b'f' => c - b'a' + 10,
        b'A'..=b'F' => c - b'A' + 10,
        _ => 0,
    };
    b.chunks(2).map(|p| if p.len() == 2 { h(p[0]) << 4 | h(p[1]) } else { h(p[0]) }).collect()
}

fn hex(b: &[u8]) -> String {
    const D: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(b.len() * 2);
    for &c in b {
        s.push(D[(c >> 4) as usize] as char);
        s.push(D[(c & 15) as usize] as char);
    }
    s
}

fn hq(b: &[u8]) -> String {
    format!("\"{}\"", hex(b))
}

fn i32s_le(v: &[i32]) -> Vec<u8> {
    v.iter().flat_map(|x| x.to_le_bytes()).collect()
}

// ---------------------------------------------------------------- generator, tables

struct Lcg(u32);

impl Lcg {
    /// One shuffle draw for a range `d` (engine-core.md section 5): one step, widened by shifting in 1 bits.
    fn shuffle_draw(&mut self, d: u32) -> u32 {
        let mut r = self.next() as u32;
        let mut m: u32 = 0x7fff;
        while m < d && m != 0xffff_ffff {
            m = (m << 15) | 0x7fff;
            r = (r << 15) | 0x7fff;
        }
        if d == 0 {
            return 0;
        }
        r % d
    }
}

impl Lcg {
    fn next(&mut self) -> i32 {
        self.0 = self.0.wrapping_mul(214013).wrapping_add(2531011);
        ((self.0 >> 16) & 0x7fff) as i32
    }
}

fn shuffle_in_place(a: &mut [i32], r: &mut Lcg) {
    for i in 1..a.len() {
        let j = r.shuffle_draw(i as u32) as usize;
        a.swap(i, j);
    }
}

fn sine_entry(a: u32) -> i32 {
    SIN_TABLE[((a >> 7) & 511) as usize] as i32
}

fn cosine_entry(a: u32) -> i32 {
    SIN_TABLE[128 + ((a >> 7) & 511) as usize] as i32
}

fn mulsh(p: i32, q: i32) -> i32 {
    (((p as i64) * (q as i64)) >> 13) as i32
}

fn rotation_matrix(a: u32, b: u32, c: u32) -> [i32; 9] {
    let (ca, sa) = (cosine_entry(a), sine_entry(a));
    let (cb, sb) = (cosine_entry(b), sine_entry(b));
    let (cc, sc) = (cosine_entry(c), sine_entry(c));
    let w = |x: i32, y: i32| x.wrapping_add(y);
    let s = |x: i32, y: i32| x.wrapping_sub(y);
    [
        mulsh(cb, cc),
        w(mulsh(sc, ca), mulsh(mulsh(sb, sa), cc)),
        s(mulsh(sa, sc), mulsh(mulsh(ca, sb), cc)),
        mulsh(cb, sc).wrapping_neg(),
        s(mulsh(ca, cc), mulsh(mulsh(sa, sb), sc)),
        w(mulsh(mulsh(ca, sb), sc), mulsh(sa, cc)),
        sb,
        mulsh(sa, cb).wrapping_neg(),
        mulsh(ca, cb),
    ]
}

// ---------------------------------------------------------------- 64-bit mantissa arithmetic (internal.h / ext.c)

#[derive(Clone, Copy)]
struct Ext {
    m: u64,
    e: i32,
}

const TOP: u64 = 1 << 63;
const ZERO_EXT: Ext = Ext { m: 0, e: 0 };

fn ext_from_u128(v: u128, e: i32, sticky: bool) -> Ext {
    if v == 0 {
        return ZERO_EXT;
    }
    let s = v.leading_zeros() as i32;
    let v = v << s;
    let hi = (v >> 64) as u64;
    let lo = v as u64;
    let mut r = Ext { m: hi, e: e - s + 64 };
    let g = lo >> 63 != 0;
    if g && (sticky || (lo << 1) != 0 || (r.m & 1) != 0) {
        r.m = r.m.wrapping_add(1);
        if r.m == 0 {
            r.m = TOP;
            r.e += 1;
        }
    }
    r
}

fn ext_of_double(d: u64) -> Ext {
    let bits = d & 0x7fff_ffff_ffff_ffff;
    let mut m = bits & 0xf_ffff_ffff_ffff;
    let mut e = (bits >> 52) as i32;
    if bits == 0 {
        return ZERO_EXT;
    }
    if e == 0 {
        e = 1;
    } else {
        m |= 1 << 52;
    }
    let sh = m.leading_zeros() as i32;
    Ext { m: m << sh, e: e - 1075 - sh }
}

fn double_of_ext(x: Ext) -> u64 {
    if x.m == 0 {
        return 0;
    }
    let mut lead = x.e + 63;
    let drop = if lead >= -1022 { 11 } else { 11 + (-1022 - lead) };
    if drop > 64 {
        return 0;
    }
    let (mut q, guard, below);
    if drop == 64 {
        q = 0u64;
        guard = x.m >> 63 != 0;
        below = (x.m << 1) != 0;
    } else {
        q = x.m >> drop;
        guard = (x.m >> (drop - 1)) & 1 != 0;
        below = (x.m & ((1u64 << (drop - 1)) - 1)) != 0;
    }
    if guard && (below || (q & 1) != 0) {
        q += 1;
    }
    if lead >= -1022 {
        if q == 1 << 53 {
            q >>= 1;
            lead += 1;
        }
        if lead > 1023 {
            return 0x7ff0_0000_0000_0000;
        }
        (((lead + 1023) as u64) << 52) | (q & 0xf_ffff_ffff_ffff)
    } else {
        q
    }
}

fn ext_mul(a: Ext, b: Ext) -> Ext {
    if a.m == 0 || b.m == 0 {
        return ZERO_EXT;
    }
    ext_from_u128((a.m as u128) * (b.m as u128), a.e + b.e, false)
}

fn ext_add(mut a: Ext, mut b: Ext) -> Ext {
    if a.m == 0 {
        return b;
    }
    if b.m == 0 {
        return a;
    }
    if b.e > a.e {
        std::mem::swap(&mut a, &mut b);
    }
    let d = a.e - b.e;
    let mut sticky = false;
    let (bh, bl): (u64, u64);
    if d >= 128 {
        bh = 0;
        bl = 0;
        sticky = true;
    } else if d >= 64 {
        bh = 0;
        bl = if d == 64 { b.m } else { b.m >> (d - 64) };
        sticky = d > 64 && (b.m & ((1u64 << (d - 64)) - 1)) != 0;
    } else if d == 0 {
        bh = b.m;
        bl = 0;
    } else {
        bh = b.m >> d;
        bl = b.m << (64 - d);
    }
    let mut lo = bl;
    let (mut hi, carry) = a.m.overflowing_add(bh);
    let mut e = a.e - 64;
    if carry {
        sticky |= lo & 1 != 0;
        lo = (lo >> 1) | (hi << 63);
        hi = (hi >> 1) | TOP;
        e += 1;
    }
    ext_from_u128(((hi as u128) << 64) | lo as u128, e, sticky)
}

fn ext_div(a: Ext, b: Ext) -> Ext {
    if a.m == 0 {
        return ZERO_EXT;
    }
    let mut ma = a.m >> 11;
    let mb = b.m >> 11;
    let mut q = 0u64;
    let mut xa = a.e + 11;
    let xb = b.e + 11;
    if ma < mb {
        ma <<= 1;
        xa -= 1;
    }
    let mut rem = ma;
    for _ in 0..64 {
        q <<= 1;
        if rem >= mb {
            rem -= mb;
            q |= 1;
        }
        rem <<= 1;
    }
    let guard = rem >= mb;
    if guard {
        rem -= mb;
    }
    let sticky = rem != 0;
    let mut r = Ext { m: q, e: xa - xb - 63 };
    if guard && (sticky || (q & 1) != 0) {
        r.m = r.m.wrapping_add(1);
        if r.m == 0 {
            r.m = TOP;
            r.e += 1;
        }
    }
    r
}

fn ext_sqrt(a: Ext) -> Ext {
    if a.m == 0 {
        return ZERO_EXT;
    }
    let odd = a.e & 1;
    let shift: i32 = if odd != 0 { 65 } else { 66 };
    let mut rem: u128 = 0;
    let mut root: u128 = 0;
    for i in 0..65 {
        let pos = 128 - 2 * i;
        let mut pair = 0u64;
        if pos - shift >= 0 && pos - shift < 64 {
            pair = (a.m >> (pos - shift)) & 3;
        } else if pos - shift == -1 {
            pair = (a.m << 1) & 3;
        }
        rem = (rem << 2) | pair as u128;
        let cand = (root << 2) | 1;
        root <<= 1;
        if rem >= cand {
            rem -= cand;
            root |= 1;
        }
    }
    let guard = root & 1 != 0;
    let sticky = rem != 0;
    let mut r = Ext { m: (root >> 1) as u64, e: (a.e - shift) / 2 + 1 };
    if guard && (sticky || (r.m & 1) != 0) {
        r.m = r.m.wrapping_add(1);
        if r.m == 0 {
            r.m = TOP;
            r.e += 1;
        }
    }
    r
}

fn bx_sqrt(bits: u64) -> u64 {
    if (bits << 1) == 0 {
        return bits;
    }
    if bits >> 63 != 0 {
        return 0xFFF8_0000_0000_0000;
    }
    let e = ((bits >> 52) & 0x7ff) as i32;
    if e == 0x7ff {
        return bits;
    }
    let mut m = bits & 0xf_ffff_ffff_ffff;
    let mut exp2: i32;
    if e == 0 {
        exp2 = -1074;
        while m & (1 << 52) == 0 {
            m <<= 1;
            exp2 -= 1;
        }
    } else {
        m |= 1 << 52;
        exp2 = e - 1075;
    }
    if exp2 & 1 != 0 {
        m <<= 1;
        exp2 -= 1;
    }
    let mut root = 0u64;
    let mut rem = 0u64;
    for i in 0..56 {
        let pair = if i <= 26 { (m >> (52 - 2 * i)) & 3 } else { 0 };
        rem = (rem << 2) | pair;
        if rem >= ((root << 2) | 1) {
            rem -= (root << 2) | 1;
            root = (root << 1) | 1;
        } else {
            root <<= 1;
        }
    }
    m = root >> 3;
    let pair = root & 7;
    if pair > 4 || (pair == 4 && (rem != 0 || (m & 1) != 0)) {
        m += 1;
    }
    if m == 1 << 53 {
        m >>= 1;
        exp2 += 2;
    }
    (((exp2 / 2 + 1049) as u64) << 52) | (m & 0xf_ffff_ffff_ffff)
}

fn bx_ftol(x: u64) -> i32 {
    let lo = x as u32;
    let hi = (x >> 32) as u32;
    let e = ((hi >> 20) & 0x7ff) as i32;
    if e < 1023 || e >= 1023 + 63 {
        return 0;
    }
    let mhi = (hi & 0xfffff) | 0x100000;
    let shift = e - 1075;
    let low: u32 = if shift >= 32 {
        0
    } else if shift >= 0 {
        lo << shift
    } else if shift > -32 {
        (lo >> -shift) | (mhi << (32 + shift))
    } else {
        mhi >> (-shift - 32)
    };
    (if hi >> 31 != 0 { 0u32.wrapping_sub(low) } else { low }) as i32
}

fn round_to_int(v: f64) -> i32 {
    if !(v > -4e18 && v < 4e18) {
        return i32::MIN;
    }
    let mut t = v as i64;
    let f = v - t as f64;
    if v >= 0.0 {
        if f > 0.5 || (f == 0.5 && (t & 1) != 0) {
            t += 1;
        }
    } else if f < -0.5 || (f == -0.5 && (t & 1) != 0) {
        t -= 1;
    }
    if t > 2147483647 || t < -2147483648 {
        return i32::MIN;
    }
    t as i32
}

/// The product of the int x and the 64-bit mantissa (hi:lo) with the sign/exponent word, rounded to 53 bits.
fn mul_ext(x: i32, mant_hi: u32, mant_lo: u32, sign_exp: u32) -> u64 {
    if x == 0 {
        return 0;
    }
    let ax: u32 = if x < 0 { 0u32.wrapping_sub(x as u32) } else { x as u32 };
    let neg = (x < 0) != ((sign_exp & 0x8000) != 0);
    let phi = ax as u64 * mant_hi as u64;
    let plo = ax as u64 * mant_lo as u64;
    let carry = phi + (plo >> 32);
    let low32 = plo & 0xffff_ffff;
    let sh = (64 - carry.leading_zeros() as i32) - 32;
    let top = (carry << (32 - sh)) | (low32 >> sh);
    let sticky = low32 & ((1u64 << sh) - 1);
    let mut r = top >> 11;
    let rem = top & 0x7ff;
    if rem > 0x400 || (rem == 0x400 && (sticky != 0 || (r & 1) != 0)) {
        r += 1;
    }
    let mut exp = (sign_exp & 0x7fff) as i32 - 16383 + sh;
    if r == 1 << 53 {
        r >>= 1;
        exp += 1;
    }
    let mut hi = (((exp + 1023) as u32) << 20) | ((r >> 32) as u32 & 0xfffff);
    if neg {
        hi |= 0x8000_0000;
    }
    ((hi as u64) << 32) | (r & 0xffff_ffff)
}

// ---------------------------------------------------------------- double-double atan2 (dd.c)

#[derive(Clone, Copy)]
struct Dd {
    hi: f64,
    lo: f64,
}

fn fb(b: u64) -> f64 {
    f64::from_bits(b)
}

fn two_sum(a: f64, b: f64) -> (f64, f64) {
    let t = a + b;
    let bb = t - a;
    (t, (a - (t - bb)) + (b - bb))
}

fn two_prod(a: f64, b: f64) -> (f64, f64) {
    let p = a * b;
    let mut t = 134217729.0 * a;
    let ah = t - (t - a);
    let al = a - ah;
    t = 134217729.0 * b;
    let bh = t - (t - b);
    let bl = b - bh;
    (p, ((ah * bh - p) + ah * bl + al * bh) + al * bl)
}

fn dd_make(hi: f64, lo: f64) -> Dd {
    let h = hi + lo;
    Dd { hi: h, lo: lo - (h - hi) }
}

fn dd_neg(a: Dd) -> Dd {
    Dd { hi: -a.hi, lo: -a.lo }
}

fn dd_add(a: Dd, b: Dd) -> Dd {
    let (s, mut e) = two_sum(a.hi, b.hi);
    let (t, f) = two_sum(a.lo, b.lo);
    e += t;
    let mut r = dd_make(s, e);
    r.lo += f;
    dd_make(r.hi, r.lo)
}

fn dd_mul(a: Dd, b: Dd) -> Dd {
    let (p, mut e) = two_prod(a.hi, b.hi);
    e += a.hi * b.lo + a.lo * b.hi;
    dd_make(p, e)
}

fn dd_mul_d(a: Dd, b: f64) -> Dd {
    let (p, mut e) = two_prod(a.hi, b);
    e += a.lo * b;
    dd_make(p, e)
}

fn dd_div(a: Dd, b: Dd) -> Dd {
    let q1 = a.hi / b.hi;
    let mut r = dd_add(a, dd_neg(dd_mul_d(b, q1)));
    let q2 = r.hi / b.hi;
    r = dd_add(r, dd_neg(dd_mul_d(b, q2)));
    let q3 = r.hi / b.hi;
    let q = dd_make(q1, q2);
    dd_add(q, dd_make(q3, 0.0))
}

fn dd_of(a: f64) -> Dd {
    Dd { hi: a, lo: 0.0 }
}

fn dd_const(w: [u64; 2]) -> Dd {
    Dd { hi: fb(w[0]), lo: fb(w[1]) }
}

fn dd_product(a: f64, b: f64) -> Dd {
    let (hi, lo) = two_prod(a, b);
    Dd { hi, lo }
}

fn dd_atan_ratio(n: f64, d: f64) -> Dd {
    if n == 0.0 {
        return dd_of(0.0);
    }
    let k = (n * 16.0 / d + 0.5) as i32;
    let k = k.clamp(0, 16);
    let num = dd_add(dd_of(16.0 * n), dd_neg(dd_product(k as f64, d)));
    let den = dd_add(dd_of(16.0 * d), dd_product(k as f64, n));
    let u = dd_div(num, den);
    let w = dd_mul(u, u);
    let mut p = dd_const(ATAN_SERIES[11]);
    for m in (0..=10).rev() {
        p = dd_add(dd_mul(p, w), dd_const(ATAN_SERIES[m]));
    }
    let atan_u = dd_mul(u, p);
    dd_add(dd_const(ATAN_SIXTEENTHS[k as usize]), atan_u)
}

fn dd_atan2_nonneg(s: f64, x: f64) -> Dd {
    let ax = if x < 0.0 { -x } else { x };
    if s == 0.0 && ax == 0.0 {
        return dd_of(0.0);
    }
    let mut a;
    if s <= ax {
        a = dd_atan_ratio(s, ax);
    } else {
        a = dd_add(dd_const(PI_PI_HALF[1]), dd_neg(dd_atan_ratio(ax, s)));
    }
    if x < 0.0 {
        a = dd_add(dd_const(PI_PI_HALF[0]), dd_neg(a));
    }
    a
}

fn round_shift(v: u64, shift: i32) -> u64 {
    if shift <= 0 {
        return v << (-shift);
    }
    if shift > 64 {
        return 0;
    }
    let (mut q, rem, half);
    if shift == 64 {
        q = 0;
        rem = v;
        half = 1u64 << 63;
    } else {
        q = v >> shift;
        rem = v & ((1u64 << shift) - 1);
        half = 1u64 << (shift - 1);
    }
    if rem > half || (rem == half && (q & 1) != 0) {
        q += 1;
    }
    q
}

fn dd_to_x87_double(v: Dd) -> u64 {
    let hb = v.hi.to_bits();
    let lb = v.lo.to_bits();
    if v.hi == 0.0 {
        return 0;
    }
    let eh = ((hb >> 52) & 0x7ff) as i32;
    let mh = (hb & ((1 << 52) - 1)) | (1 << 52);
    let n = mh << 11;
    let ex = |m: u64, e: i32| double_of_ext(Ext { m, e });
    if v.lo == 0.0 {
        return ex(n, eh - 1075 - 11);
    }
    let neg = lb >> 63 != 0;
    let el = ((lb >> 52) & 0x7ff) as i32;
    let ml = (lb & ((1 << 52) - 1)) | (1 << 52);
    let shift = (eh - 1075 - 11) - (el - 1075);
    if neg && mh == 1 << 52 {
        let r = round_shift(ml, shift + 1);
        if r == 0 {
            return ex(n, eh - 1075 - 11);
        }
        return ex(0u64.wrapping_sub(r), eh - 1075 - 12);
    }
    let r = round_shift(ml, shift);
    ex(if neg { n.wrapping_sub(r) } else { n.wrapping_add(r) }, eh - 1075 - 11)
}

fn angle_of_vector(x: i32, z: i32) -> i32 {
    if x == 0 && z == 0 {
        return 0;
    }
    let s = (x as i64).abs() as f64;
    let a = dd_to_x87_double(dd_atan2_nonneg(s, z as f64));
    let v = fb(a) * fb(RAD_TO_ANGLE16);
    let r = round_to_int(v);
    if x < 0 {
        r.wrapping_neg()
    } else {
        r
    }
}

fn acos_bits(xb: u64) -> u64 {
    let x = fb(xb);
    if x.is_nan() {
        return xb;
    }
    if !(x >= -1.0 && x <= 1.0) {
        return 0xFFF8_0000_0000_0000;
    }
    let sum = 1.0 + x;
    let dif = 1.0 - x;
    let prod = sum * dif;
    let s = fb(bx_sqrt(prod.to_bits()));
    dd_to_x87_double(dd_atan2_nonneg(s, x))
}

fn hypot_bits(xb: u64, yb: u64) -> u64 {
    let inf = 0x7ff0_0000_0000_0000u64;
    let is_inf = |b: u64| (b & 0x7fff_ffff_ffff_ffff) == inf;
    let is_nan = |b: u64| (b & 0x7fff_ffff_ffff_ffff) > inf;
    if is_inf(xb) || is_inf(yb) {
        return inf;
    }
    if is_nan(xb) {
        return xb;
    }
    if is_nan(yb) {
        return yb;
    }
    let ax = xb & 0x7fff_ffff_ffff_ffff;
    let ay = yb & 0x7fff_ffff_ffff_ffff;
    if ax == 0 && ay == 0 {
        return 0;
    }
    let big = if fb(ax) >= fb(ay) { ax } else { ay };
    let eb = ext_of_double(big);
    let ra = double_of_ext(ext_div(ext_of_double(ax), eb));
    let rb = double_of_ext(ext_div(ext_of_double(ay), eb));
    let (ea, ebb) = (ext_of_double(ra), ext_of_double(rb));
    let sum = double_of_ext(ext_add(ext_mul(ea, ea), ext_mul(ebb, ebb)));
    let root = double_of_ext(ext_sqrt(ext_of_double(sum)));
    double_of_ext(ext_mul(ext_of_double(root), eb))
}

// ---------------------------------------------------------------- sine and cosine of an angle at 64 bits (rotate_point)

type Fx = [u64; 5]; // 4 fraction limbs (2^-256 units) and one integer limb, little endian

fn fx_mul(a: &Fx, b: &Fx) -> Fx {
    let mut r = [0u64; 10];
    for i in 0..5 {
        let mut carry = 0u128;
        for j in 0..5 {
            let t = a[i] as u128 * b[j] as u128 + r[i + j] as u128 + carry;
            r[i + j] = t as u64;
            carry = t >> 64;
        }
        r[i + 5] = carry as u64;
    }
    [r[4], r[5], r[6], r[7], r[8]]
}

fn fx_div_small(a: &Fx, d: u64) -> Fx {
    let mut r = [0u64; 5];
    let mut rem = 0u128;
    for i in (0..5).rev() {
        let cur = (rem << 64) | a[i] as u128;
        r[i] = (cur / d as u128) as u64;
        rem = cur % d as u128;
    }
    r
}

fn fx_add(a: &Fx, b: &Fx) -> Fx {
    let mut r = [0u64; 5];
    let mut c = 0u128;
    for i in 0..5 {
        let t = a[i] as u128 + b[i] as u128 + c;
        r[i] = t as u64;
        c = t >> 64;
    }
    r
}

fn fx_sub(a: &Fx, b: &Fx) -> Fx {
    let mut r = [0u64; 5];
    let mut br = 0i128;
    for i in 0..5 {
        let t = a[i] as i128 - b[i] as i128 - br;
        r[i] = t as u64;
        br = if t < 0 { 1 } else { 0 };
    }
    r
}

fn fx_cmp(a: &Fx, b: &Fx) -> std::cmp::Ordering {
    for i in (0..5).rev() {
        if a[i] != b[i] {
            return a[i].cmp(&b[i]);
        }
    }
    std::cmp::Ordering::Equal
}

fn fx_zero(a: &Fx) -> bool {
    a.iter().all(|&x| x == 0)
}

fn fx_bit(a: &Fx, i: usize) -> bool {
    (a[i / 64] >> (i % 64)) & 1 != 0
}

fn fx_bitlen(a: &Fx) -> usize {
    for i in (0..5).rev() {
        if a[i] != 0 {
            return i * 64 + 64 - a[i].leading_zeros() as usize;
        }
    }
    0
}

/// (mantissa, sign/exponent word) of v, rounded to nearest even at 64 bits, as to_x87() does; `neg` sets the sign.
fn fx_to_x87(v: &Fx, neg: bool) -> (u64, u32) {
    if fx_zero(v) {
        return (0, 0);
    }
    let mut e = fx_bitlen(v) as i64;
    let shift = (e - 64) as usize;
    let mut m = 0u64;
    for k in 0..64 {
        if fx_bit(v, shift + k) {
            m |= 1 << k;
        }
    }
    let half = fx_bit(v, shift - 1);
    let mut rest = false;
    for i in 0..shift - 1 {
        if fx_bit(v, i) {
            rest = true;
            break;
        }
    }
    if half && (rest || m & 1 != 0) {
        m = m.wrapping_add(1);
        if m == 0 {
            m = 1 << 63;
            e += 1;
        }
    }
    let exp = (e - 256 - 1 + 16383) as u32;
    (m, exp | if neg { 0x8000 } else { 0 })
}

/// Sine and cosine of the double theta as ((mantissa, sign/exponent), (mantissa, sign/exponent)).
fn sincos80(theta: f64) -> ((u64, u32), (u64, u32)) {
    let bits = theta.abs().to_bits();
    let e = ((bits >> 52) & 0x7ff) as i32;
    let mant = if e == 0 { bits & 0xf_ffff_ffff_ffff } else { (bits & 0xf_ffff_ffff_ffff) | (1 << 52) };
    let ex = (if e == 0 { 1 } else { e }) - 1075;
    // x = mant * 2^ex * 2^256
    let sh = 256 + ex;
    let mut x: Fx = [0; 5];
    if sh >= 0 {
        let sh = sh as usize;
        let (limb, bit) = (sh / 64, sh % 64);
        let v = (mant as u128) << bit;
        for k in 0..2 {
            if limb + k < 5 {
                x[limb + k] = (v >> (64 * k)) as u64;
            }
        }
    }
    let x2 = fx_mul(&x, &x);
    let mut one: Fx = [0; 5];
    one[4] = 1;
    // sin = x - x^3/3! + ..., cos = 1 - x^2/2! + ...
    let (mut sp, mut sn) = (x, [0u64; 5]);
    let (mut cp, mut cn) = (one, [0u64; 5]);
    let (mut st, mut ct) = (x, one);
    let mut n = 1u64;
    loop {
        ct = fx_div_small(&fx_mul(&ct, &x2), (2 * n - 1) * (2 * n));
        st = fx_div_small(&fx_mul(&st, &x2), (2 * n) * (2 * n + 1));
        if fx_zero(&ct) && fx_zero(&st) {
            break;
        }
        if n % 2 == 1 {
            cn = fx_add(&cn, &ct);
            sn = fx_add(&sn, &st);
        } else {
            cp = fx_add(&cp, &ct);
            sp = fx_add(&sp, &st);
        }
        n += 1;
    }
    let fin = |p: &Fx, n: &Fx| -> (Fx, bool) {
        if fx_cmp(p, n) != std::cmp::Ordering::Less {
            (fx_sub(p, n), false)
        } else {
            (fx_sub(n, p), true)
        }
    };
    let (sv, sneg) = fin(&sp, &sn);
    let (cv, cneg) = fin(&cp, &cn);
    let tneg = theta < 0.0;
    (fx_to_x87(&sv, sneg != tneg), fx_to_x87(&cv, cneg))
}

fn rotate_point(angle: u32, x: i32, z: i32) -> (i32, i32) {
    let t = angle as u16 as i16;
    if angle == 0 || t == 0 {
        return (x, z);
    }
    let theta = t as f64 * fb(ANGLE16_TO_RAD);
    let ((sm, se), (cm, ce)) = sincos80(theta);
    let m = |v: i32, mant: u64, se: u32| fb(mul_ext(v, (mant >> 32) as u32, mant as u32, se));
    let xc = m(x, cm, ce);
    let zc = m(z, cm, ce);
    let xs = m(x, sm, se);
    let zs = m(z, sm, se);
    (round_to_int(xc - zs), round_to_int(zc + xs))
}

// ---------------------------------------------------------------- text to a number (80-bit emulation)

type E80 = (u128, i32);

enum Prod {
    Zero,
    Inf,
    Val(E80),
}

fn mul80(x: E80, y: E80) -> Prod {
    let (xm, ex) = x;
    let (ym, ey) = y;
    if ex == 32767 || ey == 32767 || ex + ey > 49149 {
        return Prod::Inf;
    }
    if ex + ey <= 16319 {
        return Prod::Zero;
    }
    let mut s = ex + ey;
    if ex == 0 {
        s += 1;
        if xm == 0 {
            return Prod::Zero;
        }
    }
    if ey == 0 {
        s += 1;
        if ym == 0 {
            return Prod::Zero;
        }
    }
    let xw: [u32; 5] = std::array::from_fn(|k| ((xm >> (16 * k)) & 0xffff) as u32);
    let yw: [u32; 5] = std::array::from_fn(|k| ((ym >> (16 * k)) & 0xffff) as u32);
    let mut a = [0u16; 7];
    for i in 0..5 {
        for j in 0..=(4 - i) {
            let prod = xw[i + j] * yw[4 - j];
            let n = a[i] as u32 | (a[i + 1] as u32) << 16;
            let (r, c) = n.overflowing_add(prod);
            a[i] = r as u16;
            a[i + 1] = (r >> 16) as u16;
            if c {
                a[i + 2] = a[i + 2].wrapping_add(1);
            }
        }
    }
    let mut w: u128 = 0;
    for i in 0..6 {
        w |= (a[i] as u128) << (16 * i);
    }
    let mut e = s - 16382;
    while e > 0 && (w >> 95) & 1 == 0 {
        w <<= 1;
        e -= 1;
    }
    if e <= 0 {
        e -= 1;
        let mut lost = false;
        while e < 0 {
            lost |= w & 1 != 0;
            w >>= 1;
            e += 1;
        }
        if lost {
            w |= 1;
        }
    }
    let a0 = (w & 0xffff) as u32;
    let a1 = ((w >> 16) & 1) != 0;
    let mut mant = w >> 16;
    if a0 > 0x8000 || (a0 == 0x8000 && a1) {
        mant += 1;
    }
    if mant == 1u128 << 80 {
        mant = 1u128 << 79;
        e += 1;
    }
    if e >= 32767 {
        return Prod::Inf;
    }
    Prod::Val((mant, e))
}

/// Rule R: keeps the top `keep` bits of `v` (of `total` bits); the carry is returned in the result unmasked.
fn rule_r(v: u128, total: u32, keep: u32) -> u128 {
    let drop = total - keep;
    let kept = v >> drop;
    let dropped = v & ((1u128 << drop) - 1);
    let first = (dropped >> (drop - 1)) & 1 != 0;
    let rest = dropped & ((1u128 << (drop - 2)) - 1) != 0;
    kept + (first && rest) as u128
}

fn round_target(v: Prod, neg: bool, dbl: bool) -> u64 {
    let (n, emax, emin, dlow, dbase, total): (u32, i32, i32, i32, i32, u32) =
        if dbl { (53, 1024, -1022, -1076, -1023, 64) } else { (24, 128, -126, -151, -127, 32) };
    let sign: u64 = if neg { 1u64 << (total - 1) } else { 0 };
    let infb: u64 = if dbl { 0x7ff0_0000_0000_0000 } else { 0x7f80_0000 };
    let (m, eb) = match v {
        Prod::Zero => return sign,
        Prod::Inf => return sign | infb,
        Prod::Val(x) => x,
    };
    if eb == 0 || m == 0 {
        return sign;
    }
    let mut e = eb - 16383;
    let mut kept = rule_r(m, 80, n);
    if kept == 1u128 << n {
        kept >>= 1;
        e += 1;
    }
    let fracbits = n - 1;
    if e >= emax {
        return sign | infb;
    }
    if e >= emin {
        let bias = emax - 1;
        return sign | (((e + bias) as u64) << fracbits) | (kept as u64 & ((1u64 << fracbits) - 1));
    }
    if e < dlow {
        return sign;
    }
    let w = (m << 16) >> ((dbase - e) as u32);
    let r = rule_r(w, 96, n) & ((1u128 << n) - 1);
    sign | (r >> 1) as u64
}

fn pow10_entry(neg: bool, idx: usize) -> E80 {
    let (m, e) = if neg { POW10_NEG[idx] } else { POW10_POS[idx] };
    (m, e as i32)
}

fn text_to(bytes: &[u8], dbl: bool) -> u64 {
    let len = bytes.len();
    let at = |i: usize| -> u8 { if i < len { bytes[i] } else { 0 } };
    let mut i = 0;
    if dbl {
        while i < len && ((9..=13).contains(&bytes[i]) || bytes[i] == 32) {
            i += 1;
        }
    }
    while i < len && matches!(bytes[i], b' ' | b'\t' | b'\n' | b'\r') {
        i += 1;
    }
    let mut neg = false;
    if at(i) == b'+' || at(i) == b'-' {
        neg = at(i) == b'-';
        i += 1;
    }
    if !(at(i).is_ascii_digit() || (at(i) == b'.' && at(i + 1).is_ascii_digit())) {
        return 0;
    }
    let mut digits: Vec<u8> = Vec::with_capacity(26);
    let mut p: i64 = 0;
    while at(i).is_ascii_digit() {
        let d = at(i) - b'0';
        if digits.is_empty() && d == 0 {
        } else if digits.len() < 25 {
            digits.push(d);
        } else {
            p += 1;
        }
        i += 1;
    }
    if at(i) == b'.' {
        i += 1;
        while at(i).is_ascii_digit() {
            let d = at(i) - b'0';
            if digits.is_empty() && d == 0 {
                p -= 1;
            } else if digits.len() < 25 {
                digits.push(d);
                p -= 1;
            }
            i += 1;
        }
    }
    let mut exp: i64 = 0;
    if matches!(at(i), b'e' | b'E' | b'd' | b'D') {
        let mut j = i + 1;
        let mut eneg = false;
        if at(j) == b'+' || at(j) == b'-' {
            eneg = at(j) == b'-';
            j += 1;
        }
        if at(j).is_ascii_digit() {
            while at(j).is_ascii_digit() {
                if exp <= 5200 {
                    exp = exp * 10 + (at(j) - b'0') as i64;
                    if exp > 5200 {
                        exp = 5201;
                    }
                }
                j += 1;
            }
            if eneg {
                exp = -exp;
            }
        }
    }
    if digits.len() == 25 {
        if digits[24] >= 5 {
            digits[23] += 1;
        }
        digits.pop();
        p += 1;
    }
    while digits.last() == Some(&0) {
        digits.pop();
        p += 1;
    }
    if digits.is_empty() {
        return round_target(Prod::Zero, neg, dbl);
    }
    let pp = exp + p;
    if pp > 5200 {
        return round_target(Prod::Inf, neg, dbl);
    }
    if pp < -5200 {
        return round_target(Prod::Zero, neg, dbl);
    }
    let mut d: u128 = 0;
    for &x in &digits {
        d = d * 10 + x as u128;
    }
    let bl = 128 - d.leading_zeros() as i32;
    let mut cur: E80 = (d << (80 - bl), 16383 + bl - 1);
    let ap = pp.unsigned_abs();
    let mut res = Prod::Val(cur);
    for g in 0..5 {
        let dg = ((ap >> (3 * g)) & 7) as usize;
        if dg == 0 {
            continue;
        }
        let f = pow10_entry(pp < 0, (dg - 1) + 7 * g);
        match mul80(cur, f) {
            Prod::Val(v) => {
                cur = v;
                res = Prod::Val(v);
            }
            other => {
                res = other;
                break;
            }
        }
    }
    round_target(res, neg, dbl)
}

// ---------------------------------------------------------------- C strings

fn lower(c: u8) -> u8 {
    if c.is_ascii_uppercase() { c + 32 } else { c }
}

fn at0(s: &[u8], i: usize) -> u8 {
    s.get(i).copied().unwrap_or(0)
}

fn sgn(a: i32) -> i32 {
    a.signum()
}

fn strncmp_g(a: &[u8], b: &[u8], n: usize, ci: bool) -> i32 {
    for i in 0..n {
        let (mut ca, mut cb) = (at0(a, i), at0(b, i));
        if ci {
            ca = lower(ca);
            cb = lower(cb);
        }
        if ca != cb {
            return sgn(ca as i32 - cb as i32);
        }
        if ca == 0 {
            return 0;
        }
    }
    0
}

fn is_ws(c: u8) -> bool {
    (9..=13).contains(&c) || c == 32
}

fn atoi(s: &[u8]) -> i32 {
    let mut i = 0;
    while i < s.len() && is_ws(s[i]) {
        i += 1;
    }
    let mut neg = false;
    if at0(s, i) == b'+' || at0(s, i) == b'-' {
        neg = at0(s, i) == b'-';
        i += 1;
    }
    let mut v = 0u32;
    while at0(s, i).is_ascii_digit() {
        v = v.wrapping_mul(10).wrapping_add((s[i] - b'0') as u32);
        i += 1;
    }
    (if neg { v.wrapping_neg() } else { v }) as i32
}

fn itoa(value: i32, radix: i32) -> Vec<u8> {
    if !(2..=36).contains(&radix) {
        return Vec::new();
    }
    let r = radix as u64;
    let (neg, mut v) = if radix == 10 && value < 0 { (true, (value as i64).unsigned_abs()) } else { (false, value as u32 as u64) };
    let mut out = Vec::new();
    if v == 0 {
        out.push(b'0');
    }
    while v > 0 {
        let d = (v % r) as u8;
        out.push(if d < 10 { b'0' + d } else { b'a' + d - 10 });
        v /= r;
    }
    if neg {
        out.push(b'-');
    }
    out.reverse();
    out
}

fn strtok(s: &[u8], d1: &[u8], d2: &[u8]) -> (i32, Vec<u8>, Vec<u8>) {
    let mut buf = vec![0xAAu8; 48];
    let n = s.len().min(47);
    buf[..n].copy_from_slice(&s[..n]);
    buf[n] = 0;
    let mut toks: Vec<i32> = Vec::new();
    let mut pos = 0usize;
    let mut done = false;
    let call = |buf: &mut Vec<u8>, delims: &[u8], pos: &mut usize, done: &mut bool| -> Option<(i32, i32)> {
        if *done {
            return None;
        }
        let mut st = *pos;
        while buf[st] != 0 && delims.contains(&buf[st]) {
            st += 1;
        }
        if buf[st] == 0 {
            *done = true;
            return None;
        }
        let mut en = st;
        while buf[en] != 0 && !delims.contains(&buf[en]) {
            en += 1;
        }
        if buf[en] != 0 {
            buf[en] = 0;
            *pos = en + 1;
        } else {
            *pos = en;
        }
        Some((st as i32, (en - st) as i32))
    };
    let mut count = 0;
    let mut next = call(&mut buf, d1, &mut pos, &mut done);
    let mut result_after = -1;
    while let Some((o, l)) = next {
        count += 1;
        if toks.len() < 44 {
            toks.push(o);
            toks.push(l);
        }
        if count >= 22 {
            // the 22nd token: check what ends the calls
            let nx = call(&mut buf, d2, &mut pos, &mut done);
            result_after = if nx.is_some() { -2 } else { -1 };
            break;
        }
        next = call(&mut buf, d2, &mut pos, &mut done);
    }
    toks.push(result_after);
    toks.push(0);
    let r = match call(&mut buf, d2, &mut pos, &mut done) {
        Some((o, _)) => o,
        None => -1,
    };
    toks.push(r);
    toks.push(0);
    toks.resize(48, 0);
    (count, i32s_le(&toks), buf)
}

// ---------------------------------------------------------------- sprintf

fn exact_decimal(m: u64, e: i32) -> (Vec<u8>, i32) {
    // value = digits * 10^dexp
    let mut limbs: Vec<u64> = vec![m % 1_000_000_000, (m / 1_000_000_000) % 1_000_000_000, m / 1_000_000_000_000_000_000];
    let mulsmall = |limbs: &mut Vec<u64>, f: u64| {
        let mut c = 0u64;
        for l in limbs.iter_mut() {
            let t = *l * f + c;
            *l = t % 1_000_000_000;
            c = t / 1_000_000_000;
        }
        while c > 0 {
            limbs.push(c % 1_000_000_000);
            c /= 1_000_000_000;
        }
    };
    let (mut k, f, dexp) = if e >= 0 { (e, 2u64, 0) } else { (-e, 5u64, e) };
    let (chunk, cf): (i32, u64) = if f == 2 { (29, 1 << 29) } else { (12, 244140625) };
    while k >= chunk {
        mulsmall(&mut limbs, cf);
        k -= chunk;
    }
    for _ in 0..k {
        mulsmall(&mut limbs, f);
    }
    let mut s = String::new();
    for (i, l) in limbs.iter().rev().enumerate() {
        if i == 0 {
            s.push_str(&format!("{}", l));
        } else {
            s.push_str(&format!("{:09}", l));
        }
    }
    let mut d: Vec<u8> = s.bytes().map(|c| c - b'0').collect();
    let lead = d.iter().position(|&c| c != 0).unwrap_or(d.len());
    d.drain(..lead);
    (d, dexp)
}

fn fmt_f(bits: u64, prec: usize) -> Vec<u8> {
    let neg = bits >> 63 != 0;
    let ebits = ((bits >> 52) & 0x7ff) as i32;
    let mut out = Vec::new();
    if neg {
        out.push(b'-');
    }
    if ebits == 0x7ff {
        out.push(b'1');
        if prec >= 1 {
            out.extend_from_slice(b".$");
        }
        return out;
    }
    let frac = bits & 0xf_ffff_ffff_ffff;
    let (m, e) = if ebits == 0 { (frac, -1074) } else { (frac | 1 << 52, ebits - 1075) };
    let (mut ds, mut dexp) = if m == 0 { (Vec::new(), 0) } else { exact_decimal(m, e) };
    if m == 0 && neg {
        out.clear();
    }
    // 17 significant digits, ties away
    if ds.len() > 17 {
        let up = ds[17] >= 5;
        let extra = ds.len() - 17;
        ds.truncate(17);
        dexp += extra as i32;
        if up {
            let mut i = 17;
            loop {
                if i == 0 {
                    ds.insert(0, 1);
                    ds.truncate(17);
                    dexp += 1;
                    break;
                }
                i -= 1;
                if ds[i] == 9 {
                    ds[i] = 0;
                } else {
                    ds[i] += 1;
                    break;
                }
            }
        }
    }
    // to the precision
    let cut = -(prec as i32) - dexp;
    let mut q: Vec<u8>;
    if ds.is_empty() {
        q = vec![0];
    } else if cut <= 0 {
        q = ds.clone();
        q.extend(std::iter::repeat(0).take((-cut) as usize));
    } else if cut as usize > ds.len() {
        q = vec![0];
    } else {
        let keep = ds.len() - cut as usize;
        let up = ds[keep] >= 5;
        q = ds[..keep].to_vec();
        if q.is_empty() {
            q.push(0);
        }
        if up {
            let mut i = q.len();
            loop {
                if i == 0 {
                    q.insert(0, 1);
                    break;
                }
                i -= 1;
                if q[i] == 9 {
                    q[i] = 0;
                } else {
                    q[i] += 1;
                    break;
                }
            }
        }
    }
    while q.len() < prec + 1 {
        q.insert(0, 0);
    }
    let lead = q.iter().position(|&c| c != 0).unwrap_or(q.len() - 1).min(q.len() - prec - 1);
    q.drain(..lead);
    let ip = q.len() - prec;
    for &c in &q[..ip] {
        out.push(b'0' + c);
    }
    if prec > 0 {
        out.push(b'.');
        for &c in &q[ip..] {
            out.push(b'0' + c);
        }
    }
    out
}

enum Arg {
    I(i32),
    F(u64),
    S(Vec<u8>),
}

fn sprintf(fmt: &[u8], kinds: &[u8], args: &[u8]) -> (Vec<u8>, i32) {
    let mut vals = Vec::new();
    let mut p = 0usize;
    for &k in kinds {
        match k {
            b'i' => {
                let mut b = [0u8; 4];
                for j in 0..4 {
                    b[j] = at0(args, p + j);
                }
                vals.push(Arg::I(i32::from_le_bytes(b)));
                p += 4;
            }
            b'f' => {
                let mut b = [0u8; 8];
                for j in 0..8 {
                    b[j] = at0(args, p + j);
                }
                vals.push(Arg::F(u64::from_le_bytes(b)));
                p += 8;
            }
            _ => {
                let mut b = [0u8; 4];
                for j in 0..4 {
                    b[j] = at0(args, p + j);
                }
                let l = u32::from_le_bytes(b) as usize;
                p += 4;
                let end = (p + l).min(args.len());
                vals.push(Arg::S(args[p.min(end)..end].to_vec()));
                p += l;
            }
        }
    }
    let mut vi = 0;
    let mut out: Vec<u8> = Vec::new();
    let mut i = 0;
    while i < fmt.len() {
        let c = fmt[i];
        i += 1;
        if c != b'%' {
            out.push(c);
            continue;
        }
        let (mut plus, mut zero, mut left, mut space) = (false, false, false, false);
        while i < fmt.len() && matches!(fmt[i], b'+' | b'0' | b'-' | b' ' | b'#') {
            match fmt[i] {
                b'+' => plus = true,
                b'0' => zero = true,
                b'-' => left = true,
                b' ' => space = true,
                _ => {}
            }
            i += 1;
        }
        let mut width = 0usize;
        while i < fmt.len() && fmt[i].is_ascii_digit() {
            width = (width * 10 + (fmt[i] - b'0') as usize).min(4096);
            i += 1;
        }
        let mut prec: Option<usize> = None;
        if i < fmt.len() && fmt[i] == b'.' {
            i += 1;
            let mut pr = 0usize;
            while i < fmt.len() && fmt[i].is_ascii_digit() {
                pr = (pr * 10 + (fmt[i] - b'0') as usize).min(4096);
                i += 1;
            }
            prec = Some(pr);
        }
        while i < fmt.len() && matches!(fmt[i], b'l' | b'h' | b'L') {
            i += 1;
        }
        if i >= fmt.len() {
            break;
        }
        let conv = fmt[i];
        i += 1;
        let (sign, mut body): (Vec<u8>, Vec<u8>);
        let mut zero_ok = true;
        match conv {
            b'%' => {
                out.push(b'%');
                continue;
            }
            b'd' | b'i' => {
                let v = match vals.get(vi) {
                    Some(Arg::I(v)) => *v,
                    Some(Arg::F(v)) => *v as i32,
                    _ => 0,
                };
                vi += 1;
                sign = if v < 0 { b"-".to_vec() } else if plus { b"+".to_vec() } else if space { b" ".to_vec() } else { vec![] };
                body = (v as i64).unsigned_abs().to_string().into_bytes();
            }
            b'x' | b'X' => {
                let v = match vals.get(vi) {
                    Some(Arg::I(v)) => *v as u32,
                    _ => 0,
                };
                vi += 1;
                sign = vec![];
                body = if conv == b'x' { format!("{:x}", v) } else { format!("{:X}", v) }.into_bytes();
            }
            b'c' => {
                let v = match vals.get(vi) {
                    Some(Arg::I(v)) => *v,
                    _ => 0,
                };
                vi += 1;
                sign = vec![];
                body = vec![v as u8];
                zero_ok = false;
            }
            b's' => {
                let v = match vals.get(vi) {
                    Some(Arg::S(v)) => v.clone(),
                    _ => Vec::new(),
                };
                vi += 1;
                sign = vec![];
                body = v;
                if let Some(pr) = prec {
                    body.truncate(pr);
                }
                zero_ok = false;
            }
            b'f' => {
                let v = match vals.get(vi) {
                    Some(Arg::F(v)) => *v,
                    _ => 0,
                };
                vi += 1;
                let t = fmt_f(v, prec.unwrap_or(6));
                if t.first() == Some(&b'-') {
                    sign = b"-".to_vec();
                    body = t[1..].to_vec();
                } else {
                    sign = if plus { b"+".to_vec() } else { vec![] };
                    body = t;
                }
            }
            _ => continue,
        }
        let total = sign.len() + body.len();
        if total >= width {
            out.extend_from_slice(&sign);
            out.extend_from_slice(&body);
        } else if left {
            out.extend_from_slice(&sign);
            out.extend_from_slice(&body);
            out.extend(std::iter::repeat(b' ').take(width - total));
        } else if zero && zero_ok {
            out.extend_from_slice(&sign);
            out.extend(std::iter::repeat(b'0').take(width - total));
            out.extend_from_slice(&body);
        } else {
            out.extend(std::iter::repeat(b' ').take(width - total));
            out.extend_from_slice(&sign);
            out.extend_from_slice(&body);
        }
    }
    let n = out.len() as i32;
    (out, n)
}

// ---------------------------------------------------------------- sscanf

fn sscanf(fmt: &[u8], text: &[u8]) -> (i32, [u32; 4], [u8; 64]) {
    let mut ints = [0x5a5a5a5au32; 4];
    let mut buf = [0xAAu8; 64];
    let mut ni = 0usize;
    let mut count = 0i32;
    let mut pos = 0usize;
    let len = text.len();
    let mut prev_eof = false;
    let mut f = 0usize;
    // outcome: Some(true) input failure, Some(false) matching failure
    let mut fail: Option<bool> = None;
    'outer: while f < fmt.len() && fail.is_none() {
        let c = fmt[f];
        f += 1;
        if is_ws(c) {
            while pos < len && is_ws(text[pos]) {
                pos += 1;
            }
            continue;
        }
        if c != b'%' {
            if pos >= len {
                fail = Some(true);
            } else if text[pos] == c {
                pos += 1;
            } else {
                fail = Some(false);
            }
            continue;
        }
        let conv = at0(fmt, f);
        f += 1;
        match conv {
            b'd' | b'i' | b'x' => {
                while pos < len && is_ws(text[pos]) {
                    pos += 1;
                }
                if pos >= len {
                    fail = Some(true);
                    continue;
                }
                let mut neg = false;
                if text[pos] == b'+' || text[pos] == b'-' {
                    neg = text[pos] == b'-';
                    pos += 1;
                    if pos >= len {
                        fail = Some(true);
                        continue;
                    }
                }
                let mut radix = 10u32;
                let hexd = |c: u8| (c as char).is_ascii_hexdigit();
                let mut prefix = false;
                if conv == b'd' {
                } else if text[pos] == b'0' && pos + 1 < len && (text[pos + 1] == b'x' || text[pos + 1] == b'X') {
                    prefix = true;
                } else if conv == b'x' {
                    radix = 16;
                } else if text[pos] == b'0' {
                    radix = 8;
                }
                if prefix {
                    radix = 16;
                    if pos + 2 >= len {
                        pos += 2;
                        fail = Some(true);
                        continue;
                    }
                    if !hexd(text[pos + 2]) {
                        fail = Some(false);
                        continue;
                    }
                    pos += 2;
                }
                let isdig = |c: u8| match radix {
                    10 => c.is_ascii_digit(),
                    16 => hexd(c),
                    _ => (b'0'..=b'7').contains(&c),
                };
                if !isdig(text[pos]) {
                    fail = Some(false);
                    continue;
                }
                let mut v = 0u32;
                while pos < len && isdig(text[pos]) {
                    let d = (text[pos] as char).to_digit(16).unwrap();
                    v = v.wrapping_mul(radix).wrapping_add(d);
                    pos += 1;
                }
                if neg {
                    v = v.wrapping_neg();
                }
                if ni < 4 {
                    ints[ni] = v;
                    ni += 1;
                }
                count += 1;
                prev_eof = pos >= len;
            }
            b's' => {
                while pos < len && is_ws(text[pos]) {
                    pos += 1;
                }
                if pos >= len {
                    fail = Some(true);
                    continue;
                }
                let st = pos;
                while pos < len && !is_ws(text[pos]) {
                    pos += 1;
                }
                let w = &text[st..pos];
                let n = w.len().min(63);
                buf[..n].copy_from_slice(&w[..n]);
                buf[n] = 0;
                count += 1;
                prev_eof = pos >= len;
            }
            b'[' => {
                let mut neg = false;
                if at0(fmt, f) == b'^' && f < fmt.len() {
                    neg = true;
                    f += 1;
                }
                let mut set = [false; 256];
                let mut first = true;
                while f < fmt.len() && (fmt[f] != b']' || first) {
                    first = false;
                    let a = fmt[f];
                    if f + 2 < fmt.len() && fmt[f + 1] == b'-' && fmt[f + 2] != b']' {
                        let b = fmt[f + 2];
                        for ch in a..=b {
                            set[ch as usize] = true;
                        }
                        f += 3;
                    } else {
                        set[a as usize] = true;
                        f += 1;
                    }
                }
                f += 1;
                if pos >= len {
                    fail = Some(true);
                    continue;
                }
                let st = pos;
                while pos < len && (set[text[pos] as usize] != neg) {
                    pos += 1;
                }
                if pos == st {
                    fail = Some(false);
                    continue;
                }
                let w = &text[st..pos];
                let n = w.len().min(63);
                buf[..n].copy_from_slice(&w[..n]);
                buf[n] = 0;
                count += 1;
                prev_eof = pos >= len;
            }
            b'n' => {
                if prev_eof {
                    fail = Some(false);
                    continue 'outer;
                }
                if ni < 4 {
                    ints[ni] = pos as u32;
                    ni += 1;
                }
            }
            b'%' => {
                while pos < len && is_ws(text[pos]) {
                    pos += 1;
                }
                if pos >= len {
                    fail = Some(true);
                } else if text[pos] == b'%' {
                    pos += 1;
                } else {
                    fail = Some(false);
                }
            }
            _ => fail = Some(false),
        }
    }
    let ret = if fail == Some(true) && count == 0 { -1 } else { count };
    (ret, ints, buf)
}

// ---------------------------------------------------------------- dispatch

fn call(name: &str, a: &A) -> Option<String> {
    let int = |v: i32| format!("[{}]", v);
    let r = match name {
        "sine" => int(sine_entry(a.u(0))),
        "cosine" => int(cosine_entry(a.u(0))),
        "mul_hi" => int((((a.i(0) as i64) * (a.i(1) as i64)) >> 32) as i32),
        "scaled_sine" => {
            let e = (((a.u(0).wrapping_add(32)) >> 7) & 511) as usize;
            int((((SIN_TABLE[e] as i64) * (a.i(1) as i64) + 4096) >> 13) as i32)
        }
        "scaled_cosine" => {
            let e = (((a.u(0).wrapping_add(0x4020)) >> 7) & 511) as usize;
            int((((SIN_TABLE[e] as i64) * (a.i(1) as i64) + 4096) >> 13) as i32)
        }
        "angle_of_vector" => int(angle_of_vector(a.i(0), a.i(1))),
        "rotate_point" => {
            let (x, z) = rotate_point(a.u(0), a.i(1), a.i(2));
            format!("[{}, {}]", x, z)
        }
        "rotation_matrix" => {
            let m = rotation_matrix(a.u(0), a.u(1), a.u(2));
            format!("[{}]", m.iter().map(|v| v.to_string()).collect::<Vec<_>>().join(", "))
        }
        "apply_rotation" => {
            let m: Vec<i32> = (0..9).map(|k| a.i(k)).collect();
            let (mut x, mut y, z) = (a.i(9), a.i(10), a.i(11));
            let aliased = a.i(12) == 1;
            let w = |p: i32, q: i32| mulsh(p, q);
            let rx = w(x, m[0]).wrapping_add(w(y, m[3])).wrapping_add(w(z, m[6]));
            if aliased {
                x = rx;
            }
            let ry = w(x, m[1]).wrapping_add(w(y, m[4])).wrapping_add(w(z, m[7]));
            if aliased {
                y = ry;
            }
            let rz = w(x, m[2]).wrapping_add(w(y, m[5])).wrapping_add(w(z, m[8]));
            format!("[{}, {}, {}]", rx, ry, rz)
        }
        "mul_div" => {
            let c = a.i(2) as i64;
            if c == 0 {
                int(0)
            } else {
                int(((a.i(0) as i64 * a.i(1) as i64).wrapping_div(c)) as i32)
            }
        }
        "sqrt" => format!("[{}]", bx_sqrt(a.f(0))),
        "sqrt_float" => {
            let x = f32::from_bits(a.u(0));
            let r = if x == 0.0 || x.is_nan() { x } else if x < 0.0 { f32::from_bits(0xffc0_0000) } else { x.sqrt() };
            format!("[{}]", r.to_bits())
        }
        "hypot" => format!("[{}]", hypot_bits(a.f(0), a.f(1))),
        "acos" => format!("[{}]", acos_bits(a.f(0))),
        "float_text" => format!("[{}]", text_to(&a.cs(0), false)),
        "double_text" => format!("[{}]", text_to(&a.cs(0), true)),
        "rand" => {
            let mut r = Lcg(a.u(0));
            let v: Vec<i32> = (0..10000).map(|_| r.next()).collect();
            format!("[{}]", hq(&i32s_le(&v)))
        }
        "shuffle" => {
            let n = a.i(1).clamp(0, 1 << 24) as usize;
            let mut r = Lcg(a.u(0));
            let mut v: Vec<i32> = (0..n as i32).map(|i| 3 * i + 1).collect();
            shuffle_in_place(&mut v, &mut r);
            let nx: Vec<i32> = (0..4).map(|_| r.next()).collect();
            format!("[{}, {}]", hq(&i32s_le(&v)), hq(&i32s_le(&nx)))
        }
        "load_sequence" => {
            let n = a.i(1).clamp(0, 10) as usize;
            let mut r = Lcg(a.u(0));
            for _ in 0..391606 {
                r.next();
            }
            let mut v: Vec<i32> = (0..10).collect();
            shuffle_in_place(&mut v[..n], &mut r);
            let nx: Vec<i32> = (0..4).map(|_| r.next()).collect();
            format!("[{}, {}]", hq(&i32s_le(&v)), hq(&i32s_le(&nx)))
        }
        "atoi" => int(atoi(&a.cs(0))),
        "compare_bytes" => {
            let ca = a.u(0) as u8;
            let s1: Vec<u8> = if ca == 0 { vec![] } else { vec![ca] };
            let mut o: [Vec<u8>; 5] = Default::default();
            for b in 0..=255u8 {
                let s2: Vec<u8> = if b == 0 { vec![] } else { vec![b] };
                o[0].push(strncmp_g(&s1, &s2, usize::MAX, true) as i8 as u8);
                o[1].push(strncmp_g(&s1, &s2, 1, true) as i8 as u8);
                o[2].push(strncmp_g(&s1, &s2, 2, true) as i8 as u8);
                o[3].push(strncmp_g(&s1, &s2, 1, false) as i8 as u8);
                o[4].push(strncmp_g(&s1, &s2, 2, false) as i8 as u8);
            }
            format!("[{}]", o.iter().map(|v| hq(v)).collect::<Vec<_>>().join(", "))
        }
        "ftol" => int(bx_ftol(a.f(0))),
        "isalnum" | "isdigit" | "isspace" => {
            let c = a.i(0);
            let b = if (0..256).contains(&c) { c as u8 } else { 0 };
            let v = if !(0..256).contains(&c) {
                0
            } else {
                match name {
                    "isalnum" => {
                        if b.is_ascii_digit() { 4 } else if b.is_ascii_uppercase() { 1 } else if b.is_ascii_lowercase() { 2 } else { 0 }
                    }
                    "isdigit" => if b.is_ascii_digit() { 4 } else { 0 },
                    _ => if is_ws(b) { 8 } else { 0 },
                }
            };
            int(v)
        }
        "tolower" => {
            let c = a.i(0);
            int(if (65..=90).contains(&c) { c + 32 } else { c })
        }
        "toupper" => {
            let c = a.i(0);
            int(if (97..=122).contains(&c) { c - 32 } else { c })
        }
        "itoa" => format!("[{}]", hq(&itoa(a.i(0), a.i(1)))),
        "memmove" => {
            let mut buf: Vec<u8> = (0..64u32).map(|i| ((37 * i + 11) % 256) as u8).collect();
            let (d, s, n) = (a.i(0), a.i(1), a.i(2));
            if d >= 0 && s >= 0 && n >= 0 && (d as i64 + n as i64) <= 64 && (s as i64 + n as i64) <= 64 {
                buf.copy_within(s as usize..(s + n) as usize, d as usize);
            }
            format!("[{}, {}]", d, hq(&buf))
        }
        "sprintf" => {
            let (s, n) = sprintf(&a.cs(0), &a.b(1), &a.b(2));
            format!("[{}, {}]", hq(&s), n)
        }
        "sscanf" => {
            let (r, ints, buf) = sscanf(&a.cs(0), &a.cs(1));
            let ib: Vec<u8> = ints.iter().flat_map(|x| x.to_le_bytes()).collect();
            format!("[{}, {}, {}]", r, hq(&ib), hq(&buf))
        }
        "strchr" | "strrchr" => {
            let s = a.cs(0);
            let c = a.i(1) as u8;
            let mut full = s.clone();
            full.push(0);
            let r = if name == "strchr" { full.iter().position(|&x| x == c) } else { full.iter().rposition(|&x| x == c) };
            int(r.map(|v| v as i32).unwrap_or(-1))
        }
        "strcmpi" => int(strncmp_g(&a.cs(0), &a.cs(1), usize::MAX, true)),
        "strnicmp" => int(strncmp_g(&a.cs(0), &a.cs(1), a.u(2) as usize, true)),
        "strncmp" => int(strncmp_g(&a.cs(0), &a.cs(1), a.u(2) as usize, false)),
        "strcspn" => {
            let (s, set) = (a.cs(0), a.cs(1));
            int(s.iter().position(|c| set.contains(c)).unwrap_or(s.len()) as i32)
        }
        "strstr" => {
            let (h, n) = (a.cs(0), a.cs(1));
            let r = if n.is_empty() { Some(0) } else { h.windows(n.len()).position(|w| w == &n[..]) };
            int(r.map(|v| v as i32).unwrap_or(-1))
        }
        "strlwr" => {
            let s: Vec<u8> = a.cs(0).iter().map(|&c| lower(c)).collect();
            format!("[{}]", hq(&s))
        }
        "strncpy" => {
            let s = a.cs(0);
            let n = (a.u(1) as usize).min(24);
            let mut d = vec![0xAAu8; 24];
            for i in 0..n {
                d[i] = at0(&s, i);
            }
            format!("[{}]", hq(&d))
        }
        "strtok" => {
            let (c, t, b) = strtok(&a.cs(0), &a.cs(1), &a.cs(2));
            format!("[{}, {}, {}]", c, hq(&t), hq(&b))
        }
        _ => return None,
    };
    Some(r)
}

pub fn run(calls: &[Call]) -> Vec<String> {
    calls
        .iter()
        .map(|c| {
            let a = A::parse(c.args.first().map(|s| s.as_str()).unwrap_or("[]"));
            let r = std::panic::catch_unwind(|| call(&c.name, &a)).ok().flatten();
            format!("{}\t{}", c.name, r.unwrap_or_else(|| "[]".to_string()))
        })
        .collect()
}
