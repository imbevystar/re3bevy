# Cleanup within the task

Read for source cleanup or refactoring. Correct flaws introduced by the change
and existing flaws that prevent a correct implementation. Nearby independent
cleanup is optional only when small, behavior-preserving and easy to review;
do not rewrite a module merely because you read it. Report material unrelated
issues briefly rather than folding them into the task.

- Keep one canonical definition/owner; compatibility re-exports are fine, synchronized copies are not.
- Required missing dependencies and unexpected failures need an error, existing named counter or rate-limited event. Expected range, state and visibility filtering needs no warning. Do not instrument every early return.
- Replace touched ambiguous indices, tuples or constants with names that convey their behavior. Derive values from the reference; naming an invented value does not make it correct.
- Comments explain current behavior, invariants or reference functions. Remove obsolete migration narratives; do not add task history.
- Do not hide a missing implementation behind fake values, no-ops or fallback assets. Trace the required production path and dependency first.
- Remove temporary debugging code. Keep useful diagnostics bounded and on the correct owner; avoid per-frame log spam.
- Extract functions/components when responsibility or access warrants it. File size alone is not a reason for a broad split.

Preserve behavior during cleanup. If evidence shows a behavior divergence, use
[behavior-parity.md](behavior-parity.md) for that correction. If isolated checks
pass but the game fails, inspect production inputs, ordering and lifecycle.
