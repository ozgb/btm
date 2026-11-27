# Runtime notes

- `shuffle` / `load_sequence`: engine-core.md section 5 states it (one shuffle draw per step, widened above 0x7fff); the code now follows it for every size. The records fit a cyclic (Sattolo style)
  shuffle: for i = 1 .. n-1, j = rand() % i, swap(a[i], a[j]) (all 551 shuffle records; every load_sequence one).
  The 2 records of size 32769 differ only at i = 32768, where the recorded j is 32767 (rand's maximum) whatever
  the draw (13252 and 29246); the code calls `shuffle_draw`, which gives that value through the widening of section 5.
- No other record contradicts the README rules.
