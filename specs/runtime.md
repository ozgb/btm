# runtime

## 1. Role

The runtime is the library every btm module links: the C runtime functions that game logic calls,
and the game's own fixed-point maths helpers that no subsystem spec owns. It reproduces their results
bit for bit on every host, with no dependence on the host CPU's floating-point unit: square roots,
the hypotenuse and the arc cosine, the decimal number parser, float to integer conversion, the random
streams and the shuffle that draws from them, the table sine and cosine, rotations and the scaled
integer products. It is a library, not a module: it has no slots, no state block fields and no
events, and a module calls it directly.

## 2. Interface

### Entry points

An entry is either **imported** (its code is in the kit's `runtime/` directory, MIT licensed, and
this spec gives it one line) or **specified** (this spec defines it, and the runtime crate implements
it from the definition). The imported files are `runtime/bitexact.h`, `runtime/internal.h`,
`runtime/bx_tables.h`, `runtime/sqrt.c`, `runtime/ext.c`, `runtime/dd.c`, `runtime/rand.c` and
`runtime/bitexact_tables.py` (which generates the tables that `runtime/bx_tables.h` declares). This
spec uses those files' own names (`bx_sqrt`, `bx_rand_state`, the `bx_ext` and `bx_dd` helpers, the
tables) wherever a definition needs them.

| Entry | What it is | Kind |
| --- | --- | --- |
| `sqrt` | correctly rounded double square root | imported (`bx_sqrt`, `runtime/sqrt.c`) |
| `rand`, `srand` | the linear congruential generator, one state per stream | imported (`bx_rand`, `bx_srand`, `runtime/rand.c`) |
| `_ftol` | double to 32-bit integer, truncating | specified (section 4.1) |
| `_hypot` | hypotenuse of two doubles | specified (section 4.2) |
| `acos` | arc cosine of a double | specified (section 4.3), on the helpers of `runtime/dd.c` |
| `atof` | decimal text to double | specified (section 4.4) |
| `strtof` | decimal text to float: the value a plain `%f` of `sscanf` stores | specified (section 4.4) |
| `shuffle` | the three-argument random shuffle, drawing from a named stream | specified (section 4.5) |
| `qsort` | sort by a comparator | specified (section 4.6) |
| `sine`, `cosine` | table sine and cosine of a 16-bit angle | specified (section 4.7), table imported (`bx_sin_table`) |
| `mul_hi` | high word of a 64-bit product | specified (section 4.7) |
| `scaled_sine`, `scaled_cosine` | an integer scaled by the table sine or cosine | specified (section 4.7), table imported (`bx_sin_table`) |
| `angle_of_vector` | the 16-bit angle of a vector | specified (section 4.8), constant imported (`bx_rad_to_angle16_bits`) |
| `rotate_point` | rotation of an integer point by a 16-bit angle | specified (section 4.9), tables imported (`bx_angle16_to_rad_bits`, `bx_sincos_table`) |
| `rotation_matrix` | a fixed-point 3 × 3 matrix from three angles | specified (section 4.10) |
| `apply_rotation` | a fixed-point matrix applied to a vector | specified (section 4.10) |
| `mul_div` | integer product and quotient through 64 bits | specified (section 4.11) |
| `explosion_draw_count` | the load-stream draws of the explosion frames at match load | specified (section 4.12) |
| C string and number functions (`atoi`, `sprintf`, `sscanf` other than its float read, `_itoa`, `_strcmpi`, `_strnicmp`, `_strlwr`, `tolower`, `toupper`, `isspace`, `isdigit`, `isalnum`, `strtok`, `strncpy`, `strncmp`, `strstr`, `strchr`, `strrchr`, `strcspn`, `memmove`) | | part 2 writes these |

The names in the first column are this spec's: `shuffle` and the fixed-point helpers have no C name,
and the runtime crate may name its functions as it likes.

### The manifest entry

None. The runtime is not a module: it has no manifest entry, no slots, no imports and no fuel budget.

### Events

None.

## 3. State

- **The random streams.** Each stream is one `bx_rand_state` (`runtime/bitexact.h`): a 32-bit word,
  imported with `bx_srand` and `bx_rand`. Game logic draws from two streams, `RAND_STREAM_MAIN` and
  `RAND_STREAM_LOAD` (`interfaces/rand_stream.h`), whose words the recordings name `rng.main` and
  `rng.load`; the streams live in the state block, and `GameRand` (`interfaces/services.h`) draws from
  the one it is given.
- **Hidden state.** None in this part. `strtok`'s position is specified in part 2.
- Every other entry is a pure function of its arguments.

## 4. Behaviour

Each entry is stated as its arguments and result. The notation is in section 5.

### 4.1 `_ftol`

`_ftol(v)` takes a double *v* and gives the 32-bit int whose bits are the low 32 bits of trunc(*v*) as a
64-bit two's complement integer, where trunc rounds toward zero. A *v* in the int range gives its
truncation. A *v* with |*v*| < 2^63 outside the int range gives trunc(*v*) reduced modulo 2^32 into the
int range (Q1). A NaN, an infinity, or |*v*| ≥ 2^63 gives 0.

Part 2 checks this against the ftol vectors.

### 4.2 `_hypot`

`_hypot(x, y)` takes two doubles.

| Arguments | Result |
| --- | --- |
| *x* or *y* infinite (the other may be a NaN) | +∞ |
| otherwise, *x* a NaN | *x* |
| otherwise, *y* a NaN | *y* |
| both zero (of either sign) | +0 |
| otherwise | *h* below |

With *a* = |*x*|, *b* = |*y*| and *m* = max(*a*, *b*):

- *p* = r53(r64(*a* / *m*)) and *q* = r53(r64(*b* / *m*)), where the quotients are exact before rounding;
- *s* = r53(r64(r64(*p*²) + r64(*q*²)));
- *t* = r53(r64(√*s*));
- *h* = r53(r64(*t* × *m*)).

Each r64 and r53 applies to the exact value of its argument. *h* is +∞ when r53 overflows.

### 4.3 `acos`

`acos(x)` takes a double.

| Argument | Result |
| --- | --- |
| *x* a NaN | *x* |
| |*x*| > 1, including ±∞ | the NaN with bits 0xFFF8000000000000 |
| −1 ≤ *x* ≤ 1 | *A* below |

- *s* = `bx_sqrt`(r53(r53(1 + *x*) × r53(1 − *x*))).
- *A* = `bx_dd_to_x87_double`(`bx_dd_atan2_nonneg`(*s*, *x*)): the angle atan2(*s*, *x*) in [0, π], as the
  double-double that `bx_dd_atan2_nonneg` gives, rounded to a 64-bit significand (to nearest, ties to
  even) and that result rounded to 53 bits (to nearest, ties to even).
- **Near a midpoint.** When the 64-bit significand of that first rounding ends in the 11 bits 0x3FF,
  0x400 or 0x401, the result is instead the one the kit's vectors record for *x*: 3.1c's angle there
  is the CPU's own 64-bit arctangent rounded to 53 bits, which can be one unit of the last place away
  from *A* (section 5, "Not exact").

Examples of *A*: `acos(0.5)` is 1.0471975511965976 (0x3FF0C152382D7365), `acos(−0.5)` is
2.0943951023931957 (0x4000C152382D7366), `acos(±0)` is 0x3FF921FB54442D18, `acos(1)` is +0 and
`acos(−1)` is 0x400921FB54442D18.

### 4.4 The decimal parser: `atof` and `strtof`

Game logic reads decimal numbers in two ways: `atof`, and the float conversion of `sscanf`. Only a
plain `%f` (no width, no flag, no size modifier) occurs in game logic, in `RunInitialMission`. `strtof`
in this spec is the value that `%f` stores.

**The text `atof` reads.**

- Leading white space is skipped: the characters 0x20 and 0x09 to 0x0D.
- Then the number: an optional `+` or `-`, then decimal digits with at most one `.` among them. A `.`
  needs a digit on at least one side of it.
- Then, only when a letter `e`, `E`, `d` or `D` follows, an exponent: the letter, an optional sign and
  one or more digits. A letter without a digit after it (and its sign) is not part of the number.
- Anything else ends the text. `inf`, `nan` and hexadecimal are not numbers: `0x1A` reads as 0.
- A text whose mantissa has no digit has no number, and the result is +0, also after a `-` (Q7).

**The field `%f` reads.**

- Leading white space is skipped: the same six characters as `atof`.
- The field is an optional sign, then digits with at most one `.` among them, at least one digit in
  all. Then, when `e` or `E` follows, the field also takes that letter, an optional sign and every
  digit after it, which may be none (Q8). `d` and `D` are not exponent letters.
- A field without a digit is a matching failure: nothing is stored.
- The value of the field is its text read as `atof` reads it, rounded to a float (24 bits) instead of a
  double.
- A field of more than 300 characters is outside this spec.

**The value.** Let the number have sign σ, mantissa digits (integer part and fraction part joined, the
point removed) and exponent digits.

1. *G* is the string of mantissa digits from the first nonzero digit on. When there is no nonzero digit,
   the result is zero with sign σ: `-0` gives −0 and `-0.0e9` gives −0.
2. *D* is the number of integer-part digits from the first nonzero digit on, when that digit is in the
   integer part; otherwise *D* is minus the number of zeros between the point and the first nonzero
   digit.
3. *E* is the exponent's value with its sign, its magnitude capped at 5201 (Q4); no exponent is 0.
4. **Significant digits.** When *G* has 24 digits or fewer, its trailing zeros are stripped, *N* is the
   integer the rest denote and *c* their count. When *G* has 25 or more and its 24th digit is 5 or more,
   *N* is one more than the integer its first 24 digits denote, and *c* = 24. When *G* has 25 or more and
   its 24th digit is below 5, the trailing zeros of its first 24 digits are stripped, *N* is the integer
   the rest denote and *c* their count. Digits after the 24th play no other part (Q3).
5. *P* = *E* + *D* − *c*, so that the number is *N* × 10^*P*.
6. *P* > 5200 gives infinity with sign σ. *P* < −5200 gives zero with sign σ.
7. **The 80-bit value.** *X*₀ is *N* scaled to an 80-bit significand: *X*₀ × 2^*e*₀ = *N* with
   2^79 ≤ *X*₀ < 2^80. When *P* = 0, the value is *X*₀ × 2^*e*₀. When *P* ≠ 0, the low 16 bits of *X*₀ are
   cleared first (Q5), and then the value is multiplied by one factor per nonzero octal digit of |*P*|,
   lowest digit first: the digit *d* at place *g* (|*P*| = Σ *d*·8^*g*) gives the factor
   *F* = 10^(*d*·8^*g*) for *P* > 0 and 10^(−*d*·8^*g*) for *P* < 0, rounded to an 80-bit significand,
   to nearest, ties to even. Each product is rounded as **the product rule** below, in that order.
8. Any value of 2^16384 or more after a product gives infinity with sign σ, and any value below
   2^−16382 gives zero with sign σ.
9. The value is rounded to the result format by **the final rounding** below, and σ is applied to the
   result, whether a number, zero or infinity.

**The factors in the kit.** `bx_pow10_pos` (for *P* > 0) and `bx_pow10_neg` (for *P* < 0) hold the 29
factors each: the one for digit *d* at place *g* is entry *k* = 7*g* + *d* − 1, three 32-bit words
`table[3k]`, `table[3k+1]`, `table[3k+2]` forming a 96-bit little-endian number whose low 80 bits are
a significand *S* and whose top 16 bits are the biased exponent *b*, the value being
*S* × 2^(*b* − 16383 − 79). When the low 16 bits of *S* are 0x8000 or more, *F* has the significand
*S* − 0x10000 instead. Read so, every entry equals the *F* of step 7.

**The product rule.** For a value *X* × 2^*x* and a factor *Y* × 2^*y* (each significand an 80-bit
integer from 2^79 to 2^80 − 1):

- Split *X* = Σ *X*ᵢ·2^(16*i*) and *Y* = Σ *Y*ⱼ·2^(16*j*) into five 16-bit pieces each (*i*, *j* = 0 to 4).
- *T* = Σ over the pairs with *i* + *j* ≥ 4 of *X*ᵢ·*Y*ⱼ·2^(16(*i* + *j*)). The pairs with *i* + *j* < 4 are
  left out (Q6). *T* is a multiple of 2^64; let *U* = *T* / 2^64, so 2^94 ≤ *U* < 2^96.
- When *U* < 2^95, double it and let *k* = 1; otherwise *k* = 0.
- *R* is *U* / 2^16 rounded to nearest, ties to even. When *R* = 2^80, it becomes 2^79 and the exponent
  rises by one.
- The product is *R* × 2^(*x* + *y* + 80 − *k*).

**The final rounding.** The 80-bit value *X* × 2^*s* (2^79 ≤ *X* < 2^80) becomes a float (*p* = 24,
*lo* = −127, *hi* = 128, bias 127) or a double (*p* = 53, *lo* = −1023, *hi* = 1024, bias 1023):

- *e* = *s* + 79, the binary exponent of the value.
- *K* = ⌊*X* / 2^(80 − *p*)⌋, plus one when bit 79 − *p* of *X* is set **and** *X* mod 2^(78 − *p*) ≠ 0
  (Q2). When *K* reaches 2^*p*, *K* becomes 2^(*p* − 1) and *e* rises by one.
- *e* < *lo* − *p*: zero.
- *lo* − *p* ≤ *e* ≤ *lo* (the subnormal results, Q9):
  - *W* = ⌊*X* × 2^16 / 2^(*lo* − *e*)⌋, using the *e* after the step above and the *X* before any
    rounding.
  - *J* = ⌊*W* / 2^(96 − *p*)⌋, plus one when bit 95 − *p* of *W* is set and *W* mod 2^(94 − *p*) ≠ 0;
    when *J* reaches 2^*p*, it becomes 0.
  - The result is the subnormal number whose fraction field is ⌊*J* / 2⌋, that is ⌊*J* / 2⌋ × 2^−149
    for a float and ⌊*J* / 2⌋ × 2^−1074 for a double.
- *e* ≥ *hi*: infinity.
- Otherwise: the normal number with biased exponent *e* + bias and fraction field *K* mod 2^(*p* − 1).

**Examples (each computed from these definitions).** `atof("0.1")` is 0x3FB999999999999A, `strtof("0.1")`
is 0x3DCCCCCD, `atof("1d3")` is 1000, `atof("1e")` is 1, `atof("-")` is +0, `atof("1e4294967297")` is
+∞, `atof("123456789012345678901234567")` is 0x455987BF7C563CAA. Section 6 has the quirks' examples.

### 4.5 `shuffle`

`shuffle(first, last, stream)` reorders the *n* elements from `first` up to (not including) `last`,
drawing from the named stream. Which stream each game call names is in the subsystem specs.

- *n* < 2: nothing is drawn and nothing changes.
- Otherwise, for *i* = 1, 2, ..., *n* − 1 in that order, one draw *r* (0 to 32767) is made and the element
  at position *i* is exchanged with the element at position *w* mod *i*, where *w* is:

| *i* | *w* |
| --- | --- |
| 1 to 0x7FFF | *r* |
| 0x8000 to 0x3FFFFFFF | *r* × 2^15 + 0x7FFF |
| 0x40000000 or more | (*r* mod 4) × 2^30 + 0x3FFFFFFF |

That is *n* − 1 draws in all. The element at position *i* is never exchanged with itself (Q10).

Example: with the stream's word 0 before the call (the word `bx_srand(0)` leaves) and *n* = 3, the
draws are 38 and 7719; position 1 is exchanged with position 38 mod 1 = 0 and position 2 with
7719 mod 2 = 1, so (a, b, c) becomes (b, c, a).

### 4.6 `qsort`

Game logic makes no `qsort` call. The runtime's `qsort` orders the elements ascending by the
comparator; the order of elements that compare equal is not part of this spec.

### 4.7 `sine`, `cosine`, `mul_hi`, `scaled_sine`, `scaled_cosine`

Angles are 16-bit, 0x10000 being a full turn: only the low 16 bits of an angle count, and *a* below is
the angle as an unsigned value (0 to 65535). *T* is `bx_sin_table` (640 signed 16-bit entries):
*T*[*i*] = round(sin(*i* × 2π / 512) × 8192), the sine computed in double precision and rounded to
nearest (no entry is a tie), and *T*[*i* + 512] = *T*[*i*] for *i* = 0 to 127.

| Entry | Result |
| --- | --- |
| `sine(a)` | *T*[⌊*a* / 128⌋] |
| `cosine(a)` | *T*[⌊*a* / 128⌋ + 128] |
| `mul_hi(u, v)` | ⌊*u* × *v* / 2^32⌋, *u* and *v* signed 32-bit, the product exact in 64 bits: the high word of the product |
| `scaled_sine(a, v)` | ⌊(*T*[⌊(*a* + 32) / 128⌋ mod 512] × *v* + 4096) / 8192⌋, the low 32 bits as a signed int |
| `scaled_cosine(a, v)` | `scaled_sine(a + 0x4000, v)` |

`sine` and `cosine` take the table entry below the angle (truncation); `scaled_sine` and
`scaled_cosine` take the nearest one, a tie going up. *v* is a signed 32-bit int, and the product is
exact in 64 bits. Examples: `sine(0x4000)` is 8192, `sine(0xFFFF)` is −101 and `cosine(0xFFFF)` is
8191; `scaled_sine(0x4000, 1000)` is 1000 and `scaled_cosine(0xFFFF, −12345)` is −12345.

### 4.8 `angle_of_vector`

`angle_of_vector(x, z)` takes two signed 32-bit ints and gives an int from −32768 to 32768.

- *x* = *z* = 0 gives 0.
- Otherwise the result is the integer nearest to θ × *K*, where θ = atan2(*x*, *z*) is the exact angle
  in radians in (−π, π] (the angle from the positive *z* axis toward the positive *x* axis) and *K* is the
  double 10430.37835047 (bits 0x40C45F306DC9C78A, `bx_rad_to_angle16_bits`), exactly
  2867080569611205 / 2^38 (Q11).
- For *x* ≠ 0, the result for (−*x*, *z*) is minus the result for (*x*, *z*).
- When θ × *K* lies within 2^−36 of a half-integer, the result is the one the kit's vectors record
  (section 5, "Not exact").

Examples: (0, 1) gives 0, (1, 0) gives 16384, (−1, 0) gives −16384, (1, 1) gives 8192, (0, −1) gives
32768 (π × *K* is 32767.9999999986), (−5, −100000) gives −32767.

### 4.9 `rotate_point`

`rotate_point(angle, point)` takes a 16-bit angle and a point of two signed 32-bit ints (*x*, *z*), and
replaces the point with (*x*′, *z*′). Both coordinates are read before either is written.

- An angle of 0 leaves the point unchanged.
- Otherwise, with *a* the angle as a signed value (−32768 to 32767):
  - θ = r53(*a* × *K*₂), where *K*₂ is the double 9.587379924285e−05 (bits 0x3F1921FB54442C5A,
    `bx_angle16_to_rad_bits`).
  - *C* = cos θ and *S* = sin θ, each rounded to a 64-bit significand, to nearest, ties to even. These
    are the entries of `bx_sincos_table` for the angle taken as unsigned, *u* = *a* mod 65536: words 5*u*
    and 5*u* + 1 are the high and low halves of *S*'s significand, words 5*u* + 2 and 5*u* + 3 those of *C*,
    and word 5*u* + 4 holds *S*'s sign and biased exponent in its high 16 bits and *C*'s in its low 16
    bits (x87 extended format: the value is the significand × 2^(exponent − 16383 − 63), negative when
    the sign bit 0x8000 is set).
  - *x*′ = rint(r53(r53(*x* × *C*) − r53(*z* × *S*))) and *z*′ = rint(r53(r53(*z* × *C*) + r53(*x* × *S*))),
    each product and difference exact before its rounding.
  - rint rounds to the nearest integer, ties to even. A result outside the int range is −2147483648
    (Q12).

Examples: angle 16384 turns (1000, 0) into (0, 1000); angle −32768 turns (1000, 7) into (−1000, −7);
angle 1 turns (1000000, 0) into (1000000, 96); angle 8192 turns (2147483647, 2147483647) into
(0, −2147483648).

### 4.10 `rotation_matrix`, `apply_rotation`

3.1c's game logic makes no call to either. *m*(*u*, *v*) = ⌊*u* × *v* / 8192⌋ with the product exact in
64 bits, keeping the low 32 bits as a signed int. Sums, differences and negations wrap at 32 bits.

`rotation_matrix(angles)` takes three 16-bit angles *a*₀, *a*₁, *a*₂ and gives nine ints *M*₀ to *M*₈.
With *c*ᵢ = `cosine`(*a*ᵢ) and *s*ᵢ = `sine`(*a*ᵢ):

| Entry | Value |
| --- | --- |
| *M*₀ | *m*(*c*₁, *c*₂) |
| *M*₁ | *m*(*s*₂, *c*₀) + *m*(*m*(*s*₁, *s*₀), *c*₂) |
| *M*₂ | *m*(*s*₀, *s*₂) − *m*(*m*(*c*₀, *s*₁), *c*₂) |
| *M*₃ | −*m*(*c*₁, *s*₂) |
| *M*₄ | *m*(*c*₀, *c*₂) − *m*(*m*(*s*₀, *s*₁), *s*₂) |
| *M*₅ | *m*(*m*(*c*₀, *s*₁), *s*₂) + *m*(*s*₀, *c*₂) |
| *M*₆ | *s*₁ |
| *M*₇ | −*m*(*s*₀, *c*₁) |
| *M*₈ | *m*(*c*₀, *c*₁) |

`apply_rotation(matrix, vector, out)` takes the nine ints, a vector of three ints *v*₀, *v*₁, *v*₂ and an
output of three ints *o*₀, *o*₁, *o*₂. For *i* = 0, then 1, then 2, *o*ᵢ becomes
*m*(*v*₀, *M*ᵢ) + *m*(*v*₁, *M*₃₊ᵢ) + *m*(*v*₂, *M*₆₊ᵢ), and is written before the next component is computed.
When the vector and the output are the same three ints, *o*₁ is therefore computed from the new *v*₀, and
*o*₂ from the new *v*₀ and *v*₁ (Q13).

### 4.11 `mul_div`

`mul_div(u, v, w)` takes three signed 32-bit ints.

- *w* = 0 gives 0.
- Otherwise the result is *u* × *v* / *w*, the product exact in 64 bits and the quotient truncated toward
  zero.
- A quotient outside the int range gives −2147483648 (Q14).

### 4.12 `explosion_draw_count`

`InitExplosions` (`interfaces/weapons.h`) makes one draw of the load stream per pixel of each frame of
three explosions, and none for the lens. An explosion with duration *t* that shrinks from size *a* to
size *b* has *f* = ⌊*t* / 2⌋ frames; with *h* = (*b* − *a*) / *f* truncated toward zero, frame *k*
(*k* = 0 to *f* − 1) has size *z* = *a* + *k* × *h* and makes *z*² draws (none when *z* ≤ 0).

| Explosion | *t* | *a* to *b* | Frames | Sizes | Draws |
| --- | --- | --- | --- | --- | --- |
| 1 | 24 | 64 to 8 | 12 | 64, 60, ..., 20 | 23456 |
| 2 | 30 | 128 to 16 | 15 | 128, 121, ..., 30 | 107335 |
| 3 | 30 | 200 to 32 | 15 | 200, 189, ..., 46 | 260815 |

`explosion_draw_count` is the total, **391606**. Between the seed of the load stream in `LoadMatch` and
the start-position shuffle of `AssignStartPositions`, the load stream advances by exactly these 391606
draws and no others.

### 4.13 `sqrt`, `rand`, `srand`

- `sqrt`: `bx_sqrt` (`runtime/sqrt.c`), the correctly rounded double square root: −0 gives −0, a
  negative nonzero argument gives the NaN 0xFFF8000000000000, +∞ gives +∞ and a NaN gives itself.
- `rand` and `srand`: `bx_rand` and `bx_srand` (`runtime/rand.c`), the published generator: `srand` sets
  the stream's word to the seed, and each `rand` sets it to word × 214013 + 2531011 mod 2^32 and gives
  bits 16 to 30 of the new word (0 to 32767). Which stream each game draw uses is in the subsystem specs
  and `interfaces/rand_stream.h`.

## 5. Exactness

- **Notation.** r53(*v*) is the exact real *v* rounded to a double: to nearest, ties to even, at a
  53-bit significand, with the double's subnormal range (no bit below 2^−1074 is kept) and +∞ (or −∞)
  for a magnitude that rounds to 2^1024 or more. r64(*v*) is *v* rounded to a 64-bit significand, to
  nearest, ties to even, with no exponent limit. ⌊ ⌋ is the floor. "Truncated" rounds toward zero.
- **Integers.** Arguments and results are 32-bit two's complement unless an entry says otherwise.
  Products named "exact in 64 bits" never overflow, since both factors are 32-bit.
- **The FPU mode.** 3.1c runs all game logic in one mode: 53-bit precision, round to nearest even, all
  exceptions masked. Every rounding this spec states follows from it.
- **Random draws.** Only `rand`, `shuffle` (one `rand` per draw) and the explosion frames (section 4.12)
  draw, each from the stream it is given.
- **NaNs.** Where a result is stated as a NaN with given bits, those are 3.1c's bits. The kit's ABI
  lets the engine canonicalise NaNs, so a module must not let a result depend on them.
- **Not exact.**
  - `acos` near a 53-bit midpoint and `angle_of_vector` near a half-integer (sections 4.3 and 4.8): the
    result there is the CPU's own arctangent's, which this spec can't define as arithmetic. The kit's
    vectors record it for the arguments they hold. Elsewhere the definitions are exact.
  - A `%f` field of more than 300 characters.
  - `_hypot` when *h* overflows: this spec gives +∞.

## 6. Quirks

- **Q1.** `_ftol` of a value beyond the int range but below 2^63 in magnitude keeps the low 32 bits of
  the truncated 64-bit integer: 3000000000.7 gives −1294967296. A NaN or an infinity gives 0.
- **Q2.** The decimal parser's final rounding ignores the bit just after the rounding bit, and rounds an
  exact half down: `%f` of `-93690462` gives −93690456 (0xCCB2B34B), not the nearest float −93690464.
- **Q3.** Of 25 or more significant digits, the 25th and later are dropped, and the 24th is rounded up
  when it is itself 5 or more.
- **Q4.** The exponent's magnitude is capped at 5201 before the mantissa's position is added: `1`
  followed by 5300 zeros and `e-5300` reads as 1e99 (0x547D42AEA2879F2E), not 1.
- **Q5.** When the number has a power of ten (*P* ≠ 0), its digits keep only 64 significant bits.
- **Q6.** Each product of the parser leaves out the partial products of weight below 2^64, so their
  carries into the bits it keeps are lost.
- **Q7.** A text with no digits gives +0 even after a `-`, while `-0` gives −0.
- **Q8.** The `%f` field takes a trailing `e` or `e+` with no digits as part of the number: in `"1e+x"` the
  field is `1e+` and reads as 1.
- **Q9.** Subnormal results lose their last bit, and a value just below the smallest normal number can
  read as zero:
  - `%f` of `2.2420775429197073e-45` gives 0x00000001; the nearest float is 0x00000002.
  - `%f` of `5.87747166653028351954e-39` gives +0; the nearest float is 0x00400000.
  - `atof("7.410984687618698162648531893023e-324")` gives 0x0000000000000001; the nearest double is
    0x0000000000000002.
  - `atof("1.11253692925360066e-308")` gives +0; the nearest double is 0x0008000000000000.
- **Q10.** `shuffle` exchanges position *i* with a position below *i*, never with itself: with two
  elements, the two are always exchanged.
- **Q11.** The angle constant *K* is not 32768 / π, and `angle_of_vector` can give 32768 (for example for
  *x* = 0 and *z* < 0), one more than the largest signed 16-bit angle.
- **Q12.** `rotate_point` gives −2147483648 for a coordinate whose rotated value is outside the int
  range.
- **Q13.** `apply_rotation` with the same three ints as input and output uses the components it has
  already written.
- **Q14.** `mul_div` of a quotient outside the int range: 3.1c ends with a division fault, and this spec
  gives −2147483648.

## 6a. Calls outside the runtime

The other C runtime calls that game logic makes have no result that this spec defines. Each goes through
the host interface (`interfaces/host.h`), or has no entry there yet, or has no observable result.

**Files.**

| Call | Where | Goes through |
| --- | --- | --- |
| `fopen` | `AccountBank`, the archive file reader, the screenshot writer, `CmdPrintWeights` | `host_file_open_read` (read), `host_file_create` (write, with `HOST_FILE_TEXT` for a text mode), `host_file_open_append` (append) |
| `fread`, `fwrite`, `fclose` | the same | `host_file_read`, `host_file_write`, `host_file_close` |
| `fseek` | the same | `host_file_seek` |
| `ftell` | the same | `host_file_seek` by 0 from `HOST_SEEK_CUR`, whose result is the position |
| `fprintf` | `AccountBank`'s audit file, `DumpPlayerAI` | the text as part 2's `sprintf` makes it, then `host_file_write` |
| `_filelength` | the archive file reader | `host_file_size` |
| `_chsize` | `AccountBank` | `host_file_truncate` |
| `_findfirst`, `_findnext`, `_findclose` | the archive file reader and the directory listings | `host_dir_first`, `host_dir_next`, `host_dir_close` |
| `_mkdir` | `MakeDirectoryPath` | no host entry |
| `printf` | `LzssCompress`, `LzssExpand` (their error messages) | no host entry |

**Memory.**

| Call | Where | Goes through |
| --- | --- | --- |
| `malloc`, `free` | the game's allocator, `StringRef` | `host_mem_alloc`, `host_mem_free` |
| `calloc` | `LzssCompress`, `LzssExpand` | `host_mem_alloc`, the block then set to zero |
| `realloc` | the game's allocator | no host entry |
| `_msize` | the game's allocator (the size of a block) | no host entry |

**Process, clock and environment.**

| Call | Where | Goes through |
| --- | --- | --- |
| `time` | `LoadMatch`: the seed of `RAND_STREAM_LOAD`, in wall-clock seconds | no host entry (`host_time_ms` is not the wall clock) |
| `time`, `localtime`, `gmtime`, `asctime`, `getenv` | the crash report's text, and the search for the debug help library it uses | `host_crash_report` |
| `exit` | `FatalError` | `host_fatal` |
| `exit` | `CmdDebugBreak` | none needed: the command ends with a division fault before the call |
| `_exit` | `OutOfMemoryHandler` (exit code 3) | `host_exit` |
| `raise` | `OutOfMemoryHandler` (the abort signal, before its exit) | no host entry |
| `atexit` | eight functions that make a function-local static object on first use (the debug allocator's command-line switches, the debug help library) | no host entry; no result game logic sees |

**64-bit integer helpers.** All are two's complement on 64 bits.

- `_allmul`: the product modulo 2^64.
- `_alldiv`: the signed quotient truncated toward zero; −2^63 / −1 gives −2^63. A zero divisor is outside
  this spec (3.1c ends with a fault).
- `_allshl`: the count is the low 8 bits of the shift argument, *c* = 0 to 255. *c* ≥ 64 gives 0; otherwise
  the value × 2^*c* modulo 2^64.
- `_allshr`: the same count. *c* ≥ 64 gives 0 for a value ≥ 0 and −1 for a negative one; otherwise
  ⌊value / 2^*c*⌋.

**Compression.** `SquashPack`'s method 2 and its unpacking counterpart use zlib 1.0.4: `compress` at the
default level (6) with the deflate method, a 15-bit window, memory level 8 and the default strategy, and
`uncompress`. Output written by a method-2 pack equals zlib 1.0.4's for those settings. Saved games do
not use it: they are packed with the game's own method 1, `LzssCompress` (`interfaces/files.h`). A method-2 pack is
reached only from console commands that write archives. For a valid stream, `uncompress`'s output is
fixed by the format.

**No observable result.**

- C++ library internals: the exception objects (made, asked for their text, destroyed), the deleting
  destructors of the exception and type information objects, the string storage routines, the library
  lock, and the range error a string operation raises on a bad position. Their only effect is on the
  strings' contents, which the subsystem specs state.
- `_chkstk`: the stack probe of a large stack frame.
- `_purecall`: fills the slots of abstract classes' tables; game logic never calls it.
- `_controlfp`: changes only the exception mask, and restores it, around a sound load. The FPU mode
  is stated once, in section 5.

## 7. Examples

Part 2 adds the examples from the kit's runtime/vectors.

## 8. Kit version

Part 2 fills this.
