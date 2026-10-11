# Arena automobile PreRender effects

- Arena automobile PreRender effects mutate wheel/skid history only. Wheel
calculations borrow physical input readonly; exhaust borrows physical/drive
readonly. This query must not request unrelated vitality, appearance, entry,
audio or audio-motion owners. Compatibility helpers forward to the same
component-input calculations. Use canonical CTimer frame/time values;
native exhaust runs on each PreRender call, without an invented frame mask.
Required weather/original inputs must
be observable, never replaced with fabricated dry roads or elapsed-time clocks.

For admission, capacity and active-camera inputs, read [re3-particles-glass](../../re3-particles-glass/SKILL.md).
