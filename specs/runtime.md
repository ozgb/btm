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
| `sqrt_float` | correctly rounded float square root | specified (section 4.13) |
| `atoi` | decimal text to int | specified (section 4.14) |
| `_itoa` | int to text | specified (section 4.15) |
| `_strcmpi` | comparison of two texts, ignoring case | specified (section 4.16) |
| `_strnicmp` | comparison of two texts' first *n* bytes, ignoring case | specified (section 4.17) |
| `strncmp` | comparison of two texts' first *n* bytes | specified (section 4.18) |
| `_strlwr` | a text to lower case, in place | specified (section 4.19) |
| `tolower`, `toupper` | one character to lower or upper case | specified (sections 4.20 and 4.21) |
| `isspace`, `isdigit` | character classes | specified (sections 4.22 and 4.23) |
| `strchr` | the first occurrence of a byte | specified (section 4.24) |
| `strstr` | the first occurrence of a text | specified (section 4.25) |
| `strncpy` | a copy of at most *n* bytes, with zero fill | specified (section 4.26) |
| `strtok` | the next token of a text | specified (section 4.27) |
| `sscanf` | values read from a text by a format; its `%f` is `strtof` | specified (section 4.28) |
| `sprintf` | values written as text by a format | specified (section 4.29) |

The names in the first column are this spec's: `shuffle` and the fixed-point helpers have no C name,
and the runtime crate may name its functions as it likes.

### The kit's vectors

`runtime/vectors/vectors.json` lists the vector entries, and for each its file and its input and
output fields with their types; `runtime/vectors/README.md` gives the record format. A runtime is
judged by giving every record's inputs to the entry and comparing its outputs with the record's. Each
vector entry is an entry of this spec:

| Vector entry | Spec entry |
| --- | --- |
| `sine`, `cosine`, `mul_hi`, `scaled_sine`, `scaled_cosine` | the same names (section 4.7) |
| `angle_of_vector` | `angle_of_vector` (section 4.8) |
| `rotate_point` | `rotate_point` (section 4.9) |
| `rotation_matrix`, `apply_rotation` | the same names (section 4.10) |
| `mul_div` | `mul_div` (section 4.11) |
| `sqrt`, `sqrt_float` | `sqrt` and `sqrt_float` (section 4.13) |
| `hypot` | `_hypot` (section 4.2) |
| `acos` | `acos` (section 4.3) |
| `double_text` | `atof` (section 4.4) |
| `float_text` | `strtof` (section 4.4) |
| `rand` | `rand` and `srand` (section 4.13) |
| `shuffle` | `shuffle` (section 4.5) on `rand` |
| `load_sequence` | `srand`, the draws of `explosion_draw_count` (section 4.12), then `shuffle` |
| `ftol` | `_ftol` (section 4.1) |
| `atoi` | `atoi` (section 4.14) |
| `itoa` | `_itoa` (section 4.15) |
| `strcmpi`, `strnicmp`, `strncmp` | `_strcmpi`, `_strnicmp`, `strncmp` (sections 4.16 to 4.18) |
| `compare_bytes` | `_strcmpi`, and `_strnicmp` and `strncmp` with the counts 1 and 2, on texts of at most one byte (sections 4.16 to 4.18) |
| `strlwr`, `tolower`, `toupper` | `_strlwr`, `tolower`, `toupper` (sections 4.19 to 4.21) |
| `isspace`, `isdigit` | the same names (sections 4.22 and 4.23) |
| `strchr`, `strstr`, `strncpy`, `strtok` | the same names (sections 4.24 to 4.27) |
| `sscanf`, `sprintf` | the same names (sections 4.28 and 4.29) |
| `isalnum`, `memmove`, `strcspn`, `strrchr` | the same names (section 4.30) |

The vectors' manifest leaves out `qsort` (section 4.6).

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
- **Hidden state.** `strtok`'s position (section 4.27): one place in a text, which every `strtok` call
  with a text sets and every call with a null first argument continues from. It is not in the state
  block: game logic starts each run of `strtok` calls with a text, and no run spans two ticks.
- Every other entry is a pure function of its arguments.

## 4. Behaviour

Each entry is stated as its arguments and result. The notation is in section 5.

### 4.1 `_ftol`

`_ftol(v)` takes a double *v* and gives the 32-bit int whose bits are the low 32 bits of trunc(*v*) as a
64-bit two's complement integer, where trunc rounds toward zero. A *v* in the int range gives its
truncation. A *v* with |*v*| < 2^63 outside the int range gives trunc(*v*) reduced modulo 2^32 into the
int range (Q1). A NaN, an infinity, or |*v*| ≥ 2^63 gives 0. The kit's `ftol` vectors hold these
results for every record. `specs/wind_meteors.md` and `specs/economy.md` state −2147483648 for an
out-of-range conversion, and byte-tactics-moddable#1066 settles them.

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
| *x* > 1 or *x* < −1, including ±∞ | the NaN with bits 0xFFF8000000000000 |
| −1 ≤ *x* ≤ 1 | *A* below |

- *s* = `bx_sqrt`(r53(r53(1 + *x*) × r53(1 − *x*))).
- *A* = `bx_dd_to_x87_double`(`bx_dd_atan2_nonneg`(*s*, *x*)): the angle atan2(*s*, *x*) in [0, π], as the
  double-double that `bx_dd_atan2_nonneg` gives, rounded to a 64-bit significand (to nearest, ties to
  even) and that result rounded to 53 bits (to nearest, ties to even).
- **Near a midpoint.** When the 64-bit significand of that first rounding ends in the 11 bits 0x3FF,
  0x400 or 0x401, 3.1c's result is not fixed: it can be one unit of the last place away from *A*, and
  differs between processors. *A* is the result this spec gives there, and the kit's vectors leave out
  the arguments at which the recorded result differs from *A* (section 5, "Not exact").

Examples of *A*: `acos(0.5)` is 1.0471975511965976 (0x3FF0C152382D7365), `acos(−0.5)` is
2.0943951023931957 (0x4000C152382D7366), `acos(±0)` is 0x3FF921FB54442D18, `acos(1)` is +0 and
`acos(−1)` is 0x400921FB54442D18.

### 4.4 The decimal parser: `atof` and `strtof`

Game logic reads decimal numbers in two ways: `atof`, and the float conversion of `sscanf`. Only a
plain `%f` (no width, no flag, no size modifier) occurs in game logic, in the mission's initial unit orders. `strtof`
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
- When θ × *K* lies within 2^−36 of a half-integer, the result is not specified; the kit's vectors
  hold no such argument (section 5, "Not exact").

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
three explosions. A frame of size *z* is *z* × *z* pixels and makes *z*² draws.

| Explosion | Frames | Sizes | Draws |
| --- | --- | --- | --- |
| 1 | 12 | 64, 60, ..., 20 (each 4 below the last) | 23456 |
| 2 | 15 | 128, 121, ..., 30 (each 7 below the last) | 107335 |
| 3 | 15 | 200, 189, ..., 46 (each 11 below the last) | 260815 |

`explosion_draw_count` is the total, **391606**. With three or more start positions, the load stream
advances by exactly these 391606 draws, and no others, between its seed in `LoadMatch` and the shuffle
of the start positions. The kit's `load_sequence` vectors hold three to ten start positions.

### 4.13 `sqrt`, `rand`, `srand`

- `sqrt`: `bx_sqrt` (`runtime/sqrt.c`), the correctly rounded double square root: −0 gives −0, a
  negative nonzero argument gives the NaN 0xFFF8000000000000, +∞ gives +∞ and a NaN gives itself.
- `rand` and `srand`: `bx_rand` and `bx_srand` (`runtime/rand.c`), the published generator: `srand` sets
  the stream's word to the seed, and each `rand` sets it to word × 214013 + 2531011 mod 2^32 and gives
  bits 16 to 30 of the new word (0 to 32767). Which stream each game draw uses is in the subsystem specs
  and `interfaces/rand_stream.h`.
- `sqrt_float(f)`: for a float *f* ≥ 0, the float nearest to the exact √*f* (ties to even; none occurs),
  which is also `bx_sqrt` of *f* as a double, rounded to a float. +0 gives +0.

The C functions of sections 4.14 to 4.30 use the terms of section 5, "Bytes and texts".

### 4.14 `atoi`

`atoi(s)` takes a text and gives an int.

- White space at the start of the text is skipped.
- Then an optional `+` or `-`, then the longest run of decimal digits that follows. The first byte after
  that run ends the number, whatever it is.
- The result is the run's value *N*, negated after a `-`, reduced modulo 2^32 into the int range (Q15).
  No digit (an empty text, a sign alone, a second sign, white space between the sign and the digits)
  gives 0.

Examples: `" \t\n\v\f\r 12"` and `"12abc"` give 12, `"+-7"` and `"- 5"` give 0, `"0x10"` gives 0,
`"2147483648"` gives −2147483648, `"4294967297"` gives 1, `"-2147483649"` gives 2147483647 and
`"99999999999999999999"` gives 1661992959. A byte 0xA0 is neither white space nor a digit: `"\xA0 5"`
gives 0.

### 4.15 `_itoa`

`_itoa(value, text, radix)` writes the text of an int *value* in a radix to `text`, followed by a zero
byte, and gives `text`. Game logic passes radix 10.

| Radix | The text |
| --- | --- |
| 10 | `-` for a negative *value*, then the decimal digits of its magnitude: −2147483648 gives `-2147483648` |
| 2, 8, 16 | the digits of *value* mod 2^32 (the value as unsigned 32 bits), with `a` to `f` for the hexadecimal digits 10 to 15 and no sign: −1 in radix 16 gives `ffffffff` |

There are no leading zeros: 0 gives `0`. Other radixes are outside this spec.

### 4.16 `_strcmpi`

`_strcmpi(a, b)` is cmp(*a*, *b*, ∞, lower) (section 5): −1, 0 or 1, never a difference of bytes.

Folding to lower case puts the six bytes `[`, `\`, `]`, `^`, `_` and backquote (0x5B to 0x60) below
every letter: `_` against `A` gives −1. Bytes 0x80 to 0xFF are not folded and order above every ASCII
byte: 0xE9 against 0xC9 gives 1. A text that is a prefix of the other orders first: `a` against
`a\xA0` gives −1.

### 4.17 `_strnicmp`

`_strnicmp(a, b, n)` is cmp(*a*, *b*, *n*, lower) for a count *n* ≥ 0: −1, 0 or 1. *n* = 0 gives 0, and
`AB` against `az` with *n* = 1 gives 0.

### 4.18 `strncmp`

`strncmp(a, b, n)` is cmp(*a*, *b*, *n*, the byte itself) for a count *n* ≥ 0: −1, 0 or 1, with no case
folding. `a` against `A` gives 1, and 0xE0 against backquote gives 1.

### 4.19 `_strlwr`

`_strlwr(s)` replaces every byte *c* of the text with lower(*c*), in place, and gives `s`. The length
and the terminating zero are unchanged, and bytes 0x80 to 0xFF are left as they are.

### 4.20 `tolower`

`tolower(c)` takes an int *c* from −128 to 255. It gives *c* + 32 for *c* from 65 to 90 (`A` to `Z`), and
*c* for every other argument, −128 to −1 and 128 to 255 included.

### 4.21 `toupper`

`toupper(c)` takes an int *c* from −128 to 255. It gives *c* − 32 for *c* from 97 to 122 (`a` to `z`), and
*c* for every other argument.

### 4.22 `isspace`

`isspace(c)` gives 8 for *c* = 9 to 13 and 32 (the white space of section 5), and 0 for every other *c*
from −1 to 255. Game logic passes a text's byte as a signed character: the bytes 0x00 to 0x7F arrive as
0 to 127, the byte 0xFF as −1 (result 0), and the bytes 0x80 to 0xFE as −128 to −2, where the result is
not specified (Q21).

### 4.23 `isdigit`

`isdigit(c)` gives 4 for *c* = 48 to 57 (`0` to `9`), and 0 for every other *c* from −1 to 255. Game logic
passes a byte as `isspace`'s callers do, and the result for −128 to −2 is not specified (Q21).

### 4.24 `strchr`

`strchr(s, c)` searches the text for the byte *c* mod 256 (the low 8 bits of the int *c*), with the
terminating zero counted as the text's last byte. The result is the first such byte, or null when there
is none. *c* = 0 (or 256) gives the terminating zero, at offset equal to the length; 0x161 searches for
`a` and −23 for 0xE9.

### 4.25 `strstr`

`strstr(h, n)` gives the first position of the text *h* at which the bytes of the text *n* occur, or
null when they don't. An empty *n* gives *h*'s first byte, also when *h* is empty. Bytes 0x80 to 0xFF
match only themselves.

### 4.26 `strncpy`

`strncpy(d, s, n)` writes exactly *n* bytes to `d` and gives `d`: byte *k* (*k* = 0 to *n* − 1) is byte *k*
of the text `s` while *k* is below its length *L*, and 0 from *k* = *L* on. When *L* ≥ *n* no zero byte is
written, so `d` holds no terminator from this call. Bytes from `d` + *n* on are not changed.

### 4.27 `strtok`

`strtok(s, delims)` takes a text `s`, or null to continue the text of the previous calls, and a set of
delimiter bytes, the bytes of the text `delims` (the set may change from call to call, and may be
empty). The scan starts at `s`'s first byte, or at the position (section 3) for a null `s`.

| From the scan's start | Result | Position afterwards |
| --- | --- | --- |
| only delimiters, then the terminating zero | null | that terminating zero |
| delimiters (maybe none), then a token ended by a delimiter | the token's first byte; the delimiter just after it becomes 0 | the byte after that delimiter |
| delimiters (maybe none), then a token ended by the terminating zero | the token's first byte | the terminating zero |

A token is a run of one or more bytes that are not delimiters. A run of delimiters yields no empty
token, and once a call gives null every later null-argument call gives null too. With an empty set the
whole rest of the text is one token. A null `s` before any call with a text is outside this spec.

### 4.28 `sscanf`

`sscanf(text, format, ...)` reads values from the text by the format and stores each into the next
argument. The formats of game logic are made of white space, `,`, and the conversions `%d`, `%i`,
`%f`, `%s`, `%n`, `%[a-zA-Z]`, `%[a-zA-Z0-9.]` and `%[a-zA-Z0-9_.]`; the vectors hold the formats
`" %[a-zA-Z0-9.]"`, `" %[a-zA-Z0-9_.]"`, `" %d"`, `" %d %d"`, `" %s %n"`, `"%[a-zA-Z],%i"`,
`"%[a-zA-Z],%i,%i,%i"`, `"%d"` and `"%x"`. A format is a sequence of directives: a run of white space, an ordinary byte, or a conversion
(`%d`, `%i`, `%x`, `%f`, `%s`, `%[...]` or `%n`, none with a width, a `*` or a size letter). The
directives are matched in order against the text from its first byte.

| Directive | What it reads and stores |
| --- | --- |
| white space | every white space byte at the current place; it never fails |
| an ordinary byte | that byte; any other byte is a matching failure |
| `%d` | white space is skipped; then a field: an optional sign and the longest run of decimal digits after it |
| `%i` | white space is skipped; then a field: an optional sign, then `0x` or `0X` and the longest run of hexadecimal digits after it; or else, when the field goes on with `0`, the longest run of octal digits (`0` to `7`), that `0` included; or else the longest run of decimal digits |
| `%x` | white space is skipped; then a field: an optional sign, an optional `0x` or `0X`, and the longest run of hexadecimal digits after it |
| `%f` | the field of section 4.4, stored as a float |
| `%s` | white space is skipped; then the longest run of bytes that are not white space (0x80 to 0xFF included), stored followed by a zero byte |
| `%[...]` | nothing is skipped; the longest run of bytes in the set, stored followed by a zero byte. The set is the bytes between the brackets, where *x*`-`*y* stands for every byte from *x* to *y* |
| `%n` | nothing is read; the number of bytes of the text read so far is stored as an int, and it is not counted in the result |

Hexadecimal digits are `0` to `9`, `a` to `f` and `A` to `F`. The value of a `%d`, `%i` or `%x` field is
its digits' value *N*, negated after a `-`, reduced modulo 2^32 into the int range (Q16).

- **Input failure.** The text ends where a directive needs a byte: an ordinary byte, the field of a
  conversion after its white space, a sign with nothing after it, or `0x` with nothing after it (Q17).
- **Matching failure.** A field with no digit after its sign or its `0x`, a `%s` or `%[` field of no
  byte, or an ordinary byte that differs.
- Either failure ends the scan; what was stored before it stays stored.
- **A field that runs to the end of the text** ends the scan once it is stored: the directives after it
  have no effect, a `%n` included (Q18).
- **The result** is the number of conversions stored, `%n` left out, except −1 when the scan ended with an
  input failure before any conversion was stored.

How a `%f` field behaves at the end of the text (a sign with nothing after it, a field that runs to the
end) is not covered by the vectors (section 7).

Examples: `" %s %n"` on `"abc def"` gives 1 and stores `abc` and 4; on `"abc"` it gives 1 and stores only
`abc`. `"%d"` on `"-"` gives −1, on `"-ff"` 0 and on `"4294967296"` 1, storing 0.
`"%[a-zA-Z],%i,%i,%i"` on `"q,0x10,010,-5"` gives 4 and stores `q`, 16, 8 and −5; on `""` it gives −1
and on `" "` 0. `"%x"` on `"0x"` gives −1 and on `"0xg"` 0.

### 4.29 `sprintf`

`sprintf(d, format, ...)` writes the format's text to `d`, each conversion replaced by its argument's
text, then a zero byte, and gives the number of bytes written before that zero byte. A `%c` of 0
writes a zero byte inside the text, which is counted. The conversions in the formats that reach game
state, saves or compared output are `%d`, `%i`, `%3d`, `%4d`, `%02d`, `%+d`, `%04x`, `%c`, `%s`, `%.0f`,
`%.1f`, `%1.1f` and `%%`; the vectors also hold `%02x`, `%08x`, `%04i`, `%ld`, `%04lX`, `%08lX` and
`%08lx`, which follow the same rules. Some formats come from the translation table, and they are read by the
same rules. A conversion is `%`, then flags, an optional width (decimal digits), an optional
`.` and precision, an optional size letter `l` or `h` (which changes nothing here) and the conversion
letter.

| Conversion | Its text |
| --- | --- |
| `%d`, `%i` | the decimal digits of the int's magnitude, after a `-` for a negative int; with the `+` flag, after a `+` for any other int |
| `%x`, `%X` | the hexadecimal digits of the int mod 2^32, `a` to `f` for `%x` and `A` to `F` for `%X`; no sign |
| `%c` | one byte, the int's low 8 bits |
| `%s` | the argument text's bytes, unchanged (0x80 to 0xFF included) |
| `%f` with precision *p* | the double's fixed-point text with *p* decimals (below) |
| `%%` | one `%` |

**Width.** A text (sign included) shorter than the width is padded on the left to the width: with the
digit `0` after the sign under the `0` flag, otherwise with spaces before it. A longer text is not cut.

**The fixed-point text of a finite double *v*.**

- *R* is |*v*| rounded to 17 significant decimal digits, to nearest; every decimal digit of *R* after its
  17th significant one is 0 (Q20).
- *R* is rounded to *p* decimals: when the decimal digit at place *p* + 1 after the point is 5 or more,
  *R* is cut after place *p* and one unit of place *p* is added; otherwise it is cut (Q19).
- The text is `-` when *v* is below zero (also for a *v* that rounds to zero; −0 has no `-`), then the
  integer part's digits (at least one, `0` for a value below 1), then, when *p* > 0, `.` and the *p*
  decimals.

**The text of an infinity or a NaN** is `-` when its sign bit is set, then `1` for *p* = 0 and `1.$` for
*p* = 1. A greater *p* is outside this spec.

`%e`, `%g`, the `-`, space and `#` flags, `%u`, `%o` and a precision on another conversion occur in no
format of game logic, and are outside this spec.

Examples: `"%3d"` of −1 gives ` -1` (3), `"%+d"` of 35126 gives `+35126`, `"%02x"` of −11869 gives
`ffffd1a3`, `"%1.1f"` of 0.25 gives `0.3`, `"%.1f"` of −2.5e−7 gives `-0.0`, `"%.1f"` of −0 gives `0.0`,
and `"%02x%c"` of 512118646 and 0 gives the 9 bytes `1e864f76` and a zero byte, and 9.

### 4.30 `isalnum`, `memmove`, `strcspn`, `strrchr`

- `isalnum(c)`: 4 for `0` to `9`, 1 for `A` to `Z`, 2 for `a` to `z`, and 0 for every other argument from
  −1 to 255. The result for −128 to −2 is not specified, as in Q21.
- `memmove(d, s, n)`: copies *n* bytes from *s* to *d*, with overlapping ranges copied as though through a
  separate buffer, and gives *d*.
- `strcspn(t, set)`: the length of the longest start of *t* that holds no byte of *set*; the whole length
  of *t* for an empty *set*.
- `strrchr(t, c)`: as `strchr`, but the last such byte.

## 5. Exactness

- **Notation.** r53(*v*) is the exact real *v* rounded to a double: to nearest, ties to even, at a
  53-bit significand, with the double's subnormal range (no bit below 2^−1074 is kept) and +∞ (or −∞)
  for a magnitude that rounds to 2^1024 or more. r64(*v*) is *v* rounded to a 64-bit significand, to
  nearest, ties to even, with no exponent limit. ⌊ ⌋ is the floor. "Truncated" rounds toward zero.
- **Integers.** Arguments and results are 32-bit two's complement unless an entry says otherwise.
  Products named "exact in 64 bits" never overflow, since both factors are 32-bit.
- **Random draws.** Only `rand`, `shuffle` (one `rand` per draw) and the explosion frames (section 4.12)
  draw, each from the stream it is given.
- **NaNs.** Where a result is stated as a NaN with given bits, those are 3.1c's bits. The kit's ABI
  lets the engine canonicalise NaNs, so a module must not let a result depend on them.
- **Bytes and texts.** A text is a run of bytes ended by its terminating zero byte, which is not part of
  it; its length is its number of bytes. Bytes are compared as unsigned values, 0 to 255, so 0x80 to
  0xFF order above every ASCII byte and the terminating zero below every byte. White space is the six
  bytes 0x20 and 0x09 to 0x0D; no other byte (0xA0 included) is white space. lower(*c*) is *c* + 0x20 for
  *c* from 0x41 to 0x5A (`A` to `Z`) and *c* otherwise; no byte from 0x80 to 0xFF has a case. A result
  stated as a byte of a text is that byte's address; the vectors record it as its offset from the
  text's first byte, and null as −1.
- **The comparison cmp(*a*, *b*, *n*, *f*)** of two texts *a* and *b*, over at most *n* positions (∞ for no
  limit), with a map *f* of bytes. Each text is read with its terminating zero. Let *i* be the first
  position below *n* at which *f*(*a*[*i*]) ≠ *f*(*b*[*i*]), looking no further than the first position at
  which both bytes are 0. With no such *i* the result is 0; otherwise it is −1 when *f*(*a*[*i*]) <
  *f*(*b*[*i*]) and 1 when it is greater.
- **Not exact.**
  - `acos` near a 53-bit midpoint (section 4.3): this spec gives *A*, which 3.1c's result can differ
    from by one unit of the last place. `angle_of_vector` near a half-integer (section 4.8) is not
    specified. Elsewhere the definitions are exact.
  - A `%f` field of more than 300 characters.
  - `_hypot` when *h* overflows: this spec gives +∞.
  - `sprintf`'s `%f` when |*v*| lies exactly halfway between two 17-digit decimals: this spec does not
    say which way *R* goes. One record holds such a value (`sprintf` record 5752, `"%.1f"` of
    0xC2D325A739176DD4), and its text is the same either way.
  - `isspace` and `isdigit` for −128 to −2 (Q21).

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
- **Q15.** `atoi` of a number outside the int range keeps its value modulo 2^32: `"4294967297"` gives 1
  and `"-2147483649"` gives 2147483647.
- **Q16.** `sscanf`'s `%d`, `%i` and `%x` do the same: `"%d"` on `"4294967296"` stores 0, and `"%x"` on
  `"-2147483649"` stores −1195914825.
- **Q17.** `sscanf` counts a sign, or a `0x` of `%i` or `%x`, that ends the text as an input failure, not
  a matching failure: `"%x"` on `"-"` gives −1, while on `"-m"` it gives 0; on `"0x"` it gives −1 and on
  `"0xg"` 0.
- **Q18.** A `sscanf` field that runs to the end of the text ends the scan, so a `%n` after it stores
  nothing, even after a run of white space in the format: `" %s %n"` on `"abc"` leaves the `%n`
  argument as it was, while on `"   abc   "` it stores 9.
- **Q19.** `sprintf`'s `%f` rounds a decimal half away from zero, not to even: `"%1.1f"` of 0.25 gives
  `0.3`.
- **Q20.** `sprintf`'s `%f` writes at most 17 significant digits and 0 for every digit after them:
  `"%.1f"` of −7.495023430834638e35 (0xC7620B2A3193B6B1) gives
  `-749502343083463820000000000000000000.0`, where the exact value's digits go on `...3818...`.
- **Q21.** The results of `isspace` and `isdigit` for the arguments −128 to −2 (the bytes 0x80
  to 0xFE passed as signed characters) are not specified.

## 6a. Calls outside the runtime

The other C runtime calls that game logic makes have no result that this spec defines. Each goes through
the host interface (`interfaces/host.h`), or has no entry there yet, or has no observable result.

**Files.**

| Call | Goes through |
| --- | --- |
| `fopen` | `host_file_open_read` (read), `host_file_create` (write, with `HOST_FILE_TEXT` for a text mode), `host_file_open_append` (append) |
| `fread`, `fwrite`, `fclose` | `host_file_read`, `host_file_write`, `host_file_close` |
| `fseek` | `host_file_seek` |
| `ftell` | `host_file_seek` by 0 from `HOST_SEEK_CUR`, whose result is the position |
| `fprintf` | the text as part 2's `sprintf` makes it, then `host_file_write` |
| `_filelength` | `host_file_size` |
| `_chsize` | `host_file_truncate` |
| `_findfirst`, `_findnext`, `_findclose` | `host_dir_first`, `host_dir_next`, `host_dir_close` |
| `_mkdir` | no host entry yet: byte-tactics-moddable#1065 adds one |
| `printf` | the text as part 2's `sprintf` makes it, then `host_log` |

**Memory.**

| Call | Goes through |
| --- | --- |
| `malloc`, `free` | `host_mem_alloc`, `host_mem_free` |
| `calloc` | `host_mem_alloc`, the block then set to zero |
| `realloc` | a block of the new size from `host_mem_alloc` that starts with the old block's bytes (as many as both sizes hold), and the old block given to `host_mem_free`; no game logic reads a block's address |
| `_msize` | the size the block was allocated with |

**Process, clock and environment.**

| Call | Where | Goes through |
| --- | --- | --- |
| `time` | `LoadMatch`: the seed of `RAND_STREAM_LOAD`, in wall-clock seconds | no host entry yet: byte-tactics-moddable#1065 routes it (`host_time_ms` is not the wall clock) |
| `time`, `localtime`, `gmtime`, `asctime`, `getenv` | the crash report's text, and the search for the debug help library it uses | `host_crash_report` |
| `exit` | `FatalError` | `host_fatal` |
| `exit` | the debug-break console command | none needed: the command ends with a division fault before the call |
| `_exit` | the out-of-memory handler (exit code 3) | `host_exit` |
| `raise` | the out-of-memory handler (the abort signal, before its exit) | no host entry yet: byte-tactics-moddable#1065 routes it |

**64-bit integer operations.** All are two's complement on 64 bits.

- A product: the product modulo 2^64.
- A signed quotient: truncated toward zero; −2^63 / −1 gives −2^63. A zero divisor is outside this spec
  (3.1c ends with a fault).
- A shift left by a count: only the low 8 bits of the count are used, *c* = 0 to 255. *c* ≥ 64 gives 0;
  otherwise the value × 2^*c* modulo 2^64.
- An arithmetic shift right by a count: the same *c*. *c* ≥ 64 gives 0 for a value ≥ 0 and −1 for a
  negative one; otherwise ⌊value / 2^*c*⌋.

**Compression.** The archive packer's method 2 and its unpacking counterpart use zlib 1.0.4: `compress` at the
default level (6) with the deflate method, a 15-bit window, memory level 8 and the default strategy, and
`uncompress`. Output written by a method-2 pack equals zlib 1.0.4's for those settings. Saved games do
not use it: they are packed with the game's own method 1, `LzssCompress` (`interfaces/files.h`). A method-2 pack is
reached only from console commands that write archives. For a valid stream, `uncompress`'s output is
fixed by the format.

Calls with no result that game state, saves or compared output can see are not listed.

## 7. Examples

Records are counted from 0 in file order, and each example below is its record's inputs and outputs.
Doubles and floats are given by value where that is exact, and otherwise as their bits in hexadecimal.
An offset of −1 is a null result.

**`_ftol`** (`runtime/vectors/ftol.bin`):

- record 36: 2147483648.0 gives −2147483648 (Q1).
- record 116: 2147483649.0 gives −2147483647 (Q1).
- record 62: 0x43DFFFFFFFFFFFFF (2^63 − 1024) gives −1024 (Q1).
- record 60: 2^63 gives 0; record 73: +∞ gives 0; record 82: the NaN 0x7FF8000000000000 gives 0.

**`_hypot`** (`runtime/vectors/hypot.bin`), record 39: (59, 52) gives 0x4053A943ED6F1581, one unit
below the correctly rounded length 0x4053A943ED6F1582.

**`acos`** (`runtime/vectors/acos.bin`):

- record 0: 1 gives +0.
- record 1: −1 gives 0x400921FB54442D18.
- record 2: 0 gives 0x3FF921FB54442D18.
- record 3: 0.5 gives 0x3FF0C152382D7365.
- record 4: −0.5 gives 0x4000C152382D7366.

**`atof`** (`runtime/vectors/double_text.bin`):

- record 3: `-0` gives −0.
- record 9: `+-1` gives +0 (Q7).
- record 20: `1e` gives 1.
- record 27: `1d5` gives 100000.
- record 48: `0x10` gives +0.
- record 94: `4.9406564584124654e-324` gives the double of bits 2, not 1 (Q9).
- record 116: `9007199254740993.0000000001` gives 2^53 (0x4340000000000000), not 2^53 + 2 (Q3).
- record 117: `0.1` gives 0x3FB999999999999A.

**`strtof`** (`runtime/vectors/float_text.bin`):

- record 0: `.8e16` gives 0x59E35FA9 (7999999895928832).
- record 74: `-45625983` gives 0xCC2E0C9F (−45625980), not the nearest float −45625984 (Q2).

**`sine`, `cosine`** (`runtime/vectors/sine.bin`, `runtime/vectors/cosine.bin`):

- `sine` record 0: angle 0 gives 0.
- `sine` record 16384: angle 16384 gives 8192.
- `sine` record 32768: angle 32768 gives 0.
- `cosine` record 0: angle 0 gives 8192.
- `cosine` record 16384: angle 16384 gives 0.

**`mul_hi`** (`runtime/vectors/mul_hi.bin`), record 255: (−1, 1) gives −1.

**`scaled_sine`, `scaled_cosine`**:

- `runtime/vectors/scaled_sine.bin`, record 0: angle 0 and scale 0 give 0.
- `runtime/vectors/scaled_cosine.bin`, record 891: angle 65535 and scale −1 give −1.

**`angle_of_vector`** (`runtime/vectors/angle_of_vector.bin`), record 2: *x* = 0, *z* = −1 gives 32768
(Q11).

**`rotate_point`** (`runtime/vectors/rotate_point.bin`):

- record 4: angle 0 leaves (0, −2147483648) as it is.
- record 180: angle 1 and (2147483647, 0) give (2147483637, 205887).
- record 188: angle 1 and (2147483647, −1000000) give (−2147483648, −794113) (Q12).

**`rotation_matrix`** (`runtime/vectors/rotation_matrix.bin`), record 0: the angles (0, 0, 0) give the
matrix with 8192 on its diagonal and 0 elsewhere.

**`apply_rotation`** (`runtime/vectors/apply_rotation.bin`), record 10: the matrix (−2903, 265, 8020,
1006, −6968, −6042, −7008, 482, 7588), the vector (1000000, 65536, 1) and *aliased* = 1 give
(−346324, −66948, −289676).

**`mul_div`** (`runtime/vectors/mul_div.bin`), record 7: (0, 1, 0) gives 0.

**`sqrt`, `sqrt_float`**:

- `runtime/vectors/sqrt.bin`, record 0: 0x1CF353F81C391EAD gives 0x2E7195E017A4C91F.
- `runtime/vectors/sqrt_float.bin`, record 0: 0x04393D99 gives 0x21D9C3D7.

**`srand`, `rand`** (`runtime/vectors/rand.bin`):

- record 0: seed 0 gives the draws 38, 7719, 21238, 2437, … .
- record 1: seed 1 gives 41, 18467, 6334, 26500, … .

**`shuffle`** (`runtime/vectors/shuffle.bin`):

- record 40: seed 0 and the five elements 1, 4, 7, 10, 13 give the order 4, 13, 1, 7, 10, and the next
  four draws are 8855, 11797, 8365, 32285.
- record 0: seed 0 and no element leave the stream unchanged: the next four draws are 38, 7719, 21238,
  2437.

**`explosion_draw_count`** (`runtime/vectors/load_sequence.bin`), record 0: seed 0, 391606 draws, then the
shuffle of 0, 1, 2 gives 1, 2, 0, and the next four draws are 15442, 19688, 28255, 19395.

**`atoi`** (`runtime/vectors/atoi.bin`):

- record 7: `2147483648` gives −2147483648 (Q15).
- record 11: `4294967297` gives 1 (Q15).
- record 62: `-2147483649` gives 2147483647 (Q15).
- record 83: `+-7` gives 0.
- record 815: the byte 0x0B then `-12ab` gives −12.

**`_itoa`** (`runtime/vectors/itoa.bin`):

- record 603: −2147483648 in radix 10 gives `-2147483648`.
- record 701: −1 in radix 16 gives `ffffffff`.
- record 724: −305419896 in radix 8 gives `35562724610`.

**`_strcmpi`** (`runtime/vectors/strcmpi.bin`):

- record 618: `_` against `A` gives −1.
- record 970: `ab` against `AB` gives 0.
- record 2680: 0xE9 against 0xC9 gives 1.
- record 88: `a` against `a` and 0xA0 gives −1.

**`_strnicmp`** (`runtime/vectors/strnicmp.bin`):

- record 4814: `ab` against `aZ` with *n* = 0 gives 0.
- record 5118: `AB` against `az` with *n* = 1 gives 0.
- record 10102: 0xE0 against backquote with *n* = 1 gives 1.

**`strncmp`** (`runtime/vectors/strncmp.bin`):

- record 282: `a` against `A` with *n* = 1 gives 1.
- record 10102: 0xE0 against backquote with *n* = 1 gives 1.

**`_strcmpi`, `_strnicmp`, `strncmp` on texts of at most one byte** (`runtime/vectors/compare_bytes.bin`):
record *a* holds, for each second text from the empty one to the byte 255, one signed byte per
comparison of the text of the byte *a* (record 0: the empty text) against it.

- record 95 (`_`): against `A`, `_strcmpi` and `_strnicmp` give −1 and `strncmp` gives 1.
- record 65 (`A`): against `a`, `_strcmpi` and `_strnicmp` give 0 and `strncmp` gives −1.
- record 0 (the empty text): against `A`, every comparison gives −1.

**`_strlwr`** (`runtime/vectors/strlwr.bin`):

- record 256: ``ABCXYZ[\]^_`abcxyz@{`` gives ``abcxyz[\]^_`abcxyz@{``.
- record 258: the bytes C0 C9 DE DF E0 FF 80 A0 are unchanged.

**`tolower`, `toupper`**:

- `runtime/vectors/tolower.bin`, record 193: 65 gives 97; record 65: −63 gives −63.
- `runtime/vectors/toupper.bin`, record 250: 122 gives 90; record 361: 233 gives 233.

**`isspace`, `isdigit`**:

- `runtime/vectors/isspace.bin`, record 12: 11 gives 8; record 161: 160 gives 0; record 0: −1 gives 0.
- `runtime/vectors/isdigit.bin`, record 56: 55 gives 4; record 0: −1 gives 0.

**`strchr`** (`runtime/vectors/strchr.bin`):

- record 156: `aaa` and 0 give offset 3.
- record 18: `a` and 353 (0x161) give offset 0.
- record 41: the byte 0xE9 and −23 give offset 0.

**`strstr`** (`runtime/vectors/strstr.bin`):

- record 160: `aa` and the empty needle give offset 0.
- record 1363: the text `ab` followed by the byte 0xE9, with the needle 0xE9, gives offset 2.

**`strncpy`** (`runtime/vectors/strncpy.bin`), into 24 bytes of 0xAA:

- record 81: `ABC` with *n* = 6 leaves `ABC`, three zero bytes and eighteen 0xAA.
- record 78: `ABC` with *n* = 3 leaves `ABC` and twenty-one 0xAA, with no terminator.
- record 128: `ABCDE` with *n* = 3 leaves `ABC` and twenty-one 0xAA.

**`strtok`** (`runtime/vectors/strtok.bin`):

- record 288: `,a,,b,` with `,` gives the tokens `a` at offset 1 and `b` at offset 4, then null, and a
  further call gives null.
- record 243: `  a  b  ` with a space gives `a` at offset 2 and `b` at offset 5, then null.
- record 363: `abc` with the empty set gives `abc` at offset 0, then null.

**`sscanf`** (`runtime/vectors/sscanf.bin`):

- record 293: `" %s %n"` on `"abc def"` gives 1 and stores `abc` and 4.
- record 259: the same on `"abc"` gives 1 and stores `abc`; the `%n` stores nothing (Q18).
- record 295: the same on `"   abc   "` gives 1 and stores `abc` and 9.
- record 424: `"%[a-zA-Z],%i,%i,%i"` on `"q,0x10,010,-5"` gives 4 and stores `q`, 16, 8 and −5.
- record 366: the same format on `""` gives −1; record 367: on `" "` it gives 0.
- record 435: `"%d"` on `"-"` gives −1 (Q17); record 466: on `"-ff"` it gives 0.
- record 459: `"%d"` on `"4294967296"` gives 1 and stores 0 (Q16).
- record 494: `"%x"` on `"-1"` stores −1; record 518: on `"-2147483649"` it stores −1195914825 (Q16).
- record 528: `"%x"` on `"0x"` gives −1 (Q17); record 529: on `"0xg"` it gives 0; record 1241: on a text
  starting with `-` and the byte 0x0B it gives 0.

**`sprintf`** (`runtime/vectors/sprintf.bin`):

- record 2: `"%3d"` of −1 gives ` -1` and 3.
- record 320: `"%+d"` of 44859 gives `+44859` and 6; record 172: of 0 it gives `+0` and 2.
- record 1996: `"%04i"` of −1 gives `-001` and 4.
- record 1436: `"%02x"` of −11869 gives `ffffd1a3` and 8.
- record 2845: `"%1.1f"` of 0.25 gives `0.3` and 3 (Q19).
- record 1086: `"%.1f"` of −2.5e−7 gives `-0.0` and 4; record 644: of −0 it gives `0.0` and 3.
- record 6195: `"+%.1f"` of −1711014522793811 gives `+-1711014522793811.0` and 20.
- record 6493: `"Send - %1.1f K/s"` of 0x44B2CC217ED0D762 gives
  `Send - 88768146078764160000000.0 K/s` and 36 (Q20).
- record 674: `"%.1f"` of +∞ gives `1.$` and 3; record 2182: `"%.0f"` of +∞ gives `1` and 1.
- record 163: `"%s"` of the bytes E9 FF 80, `abc` and A0 copies them unchanged, and gives 7.
- record 1581: `"%c"` of 0 gives a zero byte and 1.

**What the vectors don't cover.** These cases are left to the definitions above, or are outside this
spec:

- `qsort` (section 4.6), and the order in which it leaves equal elements.
- `isspace`, `isdigit` and `isalnum` for −128 to −2 (Q21).
- `acos` near a 53-bit midpoint and `angle_of_vector` near a half-integer (section 5).
- `sprintf`: a `%f` value halfway between two 17-digit decimals whose text depends on which way it
  rounds, and an infinity or a NaN with a precision above 1.
- `sscanf`'s `%f`: its field at the end of the text (section 4.28), and `%f` inside a format with other
  conversions.
- `_itoa` in a radix other than 2, 8, 10 and 16.
- `strtok` with a null text before any call with a text.
- `_hypot` of an infinite or NaN argument, and `sqrt` of a negative argument.

## 8. Kit version

Kit `interfaces-v33` and `reference-traces-v34`, with the `runtime/` directory (vectors format 1) that
byte-tactics-moddable#1060 adds; the first kit tag that carries it replaces this line.
