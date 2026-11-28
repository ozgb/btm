# Spec questions

1. The spec (section 8) names kit `interfaces-v72`; the gate's verdicts (req1, req2) report `interfaces-v73`.
   The branch `spec-economy-v72` has no newer commit here (the remote could not be fetched). Is the spec
   current for v73? No recording behaviour contradicted the spec, because no verdict on behaviour was obtained.

   **Answered (by the spec writer, not shown by the kit):** the spec is current. The spec writer checked that v73 differs from v72 only in the gate's tooling and docs. The kit itself carries no change list; `interfaces/economy.h` still says version 5.0. The spec's pin stays at v72.
