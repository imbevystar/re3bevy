---
name: parity-lab
description: Compare the same input between re3 C++ and production re3bevy.
---

- Use this skill only when parity work has been selected; define a narrow observable surface.
- For the same input, capture output from both C++ re3 and production Rust. Rust-authored expected values are not parity evidence.
- Output must be deterministic and support field-level diffs; record the input, build branch, asset fingerprint, and ordering information.
- Compare integers, enums, booleans, strings, ordering, and IDs exactly. Justify float tolerances from C++ numeric behavior and precision.
- Keep the C++ harness outside `references/re-GTA`, and make the Rust harness call production APIs; do not write a second Rust copy of the behavior.
- Run only the requested case/command. A case result is evidence only for the surface, input, branch, and time interval it measures; screenshots are not parity.
