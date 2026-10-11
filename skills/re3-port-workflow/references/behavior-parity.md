# Behavior investigation and scope

Read for bug fixes, missing behavior and ports. Reproduce the reported behavior
and understand its complete production path before choosing a fix. Investigation
breadth does not automatically expand the implementation to every nearby defect.

## Trace the affected path

1. Identify the expected behavior from relevant re3 declarations, implementations and call sites; identify where Rust executes it in the production game.
2. Follow inputs through loading, creation, update, cadence/order, presentation and retirement where they affect this behavior. Check save/load or SCM interfaces when involved. Trace callers and consumers across domain boundaries; load their skills when necessary.
3. Compare the required decisions and side effects on both sides. Track verified matches, divergences, missing/stubbed paths and duplicated owners in working analysis. Source inspection and runtime evidence support different claims; record uncertainty rather than guessing.
4. Fix all causes necessary for the requested behavior to work end to end. If the first fix exposes another failure on that path, continue. Include required lifecycle/interface corrections; do not merely bypass the first failing gate.
5. Recheck downstream effects and run the affected production scenario. Use existing deterministic comparisons when they cover the behavior.

For example, a vehicle lamp failure may require tracing DFF lamp positions,
model residency, damage state and draw submission. It does not automatically
require rewriting vehicle AI, gear changes and radio playback. Expand the work
when evidence connects another owner to the failure.

## Boundaries

- A task to port or audit a whole subsystem covers that full subsystem and its affected interfaces. A specific bug covers its affected behavior path and all necessary dependencies. Neither scope allows invented behavior or disconnected implementations.
- Preserve GTA III behavior while changing ownership to native ECS. Different C++ and Rust function/class layouts are not themselves parity gaps.
- Fix required dependencies without repeated approval requests. Unrelated defects or larger architectural opportunities belong in a brief remaining-issues note, not an unsolicited refactor.
- Do not leave placeholders on the implemented path. If required behavior cannot be completed, identify the missing behavior/reference function and the blocker; do not label a partial workaround a fix.

## Completion evidence

The requested behavior works end to end in the production game; related ordering,
lifecycle and boundary effects remain correct; validation covers the actual
changed path. Report material gaps and limits of evidence explicitly. A successful
compile, screenshot or isolated case is not proof of whole-subsystem parity.

Keep investigation in working analysis. Report findings briefly in conversation;
write an audit document only if requested. Validation details are in
[the workflow](../SKILL.md).
