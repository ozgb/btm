# Spec questions

## Q-probe-1: the probe never ends when a search has no goal cell and the aim cell is stale

Spec: section 4 "The start of a search", step 7 ("when there are none, `goalX`, `goalY` keep their values"); "The
probe", step 2; section 7 "Edges" ("The probe has no limit of its own on its turns").

A request whose goal gives no goal cell (kind 0 or above 3, or a rectangle with `left` > `right` or `top` > `bottom`)
keeps the aim cell of an earlier search. If that cell is interior and passable, the greedy walk reaches it, then picks
direction 0 (neither `goalX` < x, `goalX` > x nor `goalY` > y), steps off and back, and repeats. `best` never reaches 0
and the wall walk is never entered, so `Step` runs out of fuel (a trap, ABI section 13). Reproduced by the reviewer in
local fuzzing (goal rectangle left 13, right 1, top 8, bottom 4, aim (10,14)).

The module follows the text, so it hangs in that case. Is that the reference's behaviour (the case is then outside
the contract and should be said so), or does the reference stop or fail the search some other way? No recording
reaches it, so the module keeps the text until the spec says.
