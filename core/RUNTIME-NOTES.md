# Runtime notes

- `shuffle` / `load_sequence`: the README does not state the algorithm. The records fit a cyclic (Sattolo style)
  shuffle: for i = 1 .. n-1, j = rand() % i, swap(a[i], a[j]) (549 of 551 shuffle records; every load_sequence one).
  The 2 records of size 32769 differ only at i = 32768, where the recorded j is 32767 (rand's maximum) whatever
  the draw (13252 and 29246); the code uses j = 32767 for i > 32767, one draw consumed as usual.
- No other record contradicts the README rules.
