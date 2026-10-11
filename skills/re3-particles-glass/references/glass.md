# Falling glass ownership and sequence lifetime

- CFallingGlassPane::Update's static fragment-frame sequence belongs to one
app-lifetime GlassDebrisSequence Resource. CGlass clear/initialization or level
replacement must not reset it. GtaGlassPlugin owns its initialization and the
unchanged native Glass process slot; GamePlugin composes this domain.
Live falling glass panes belong to CFallingGlassPane Components. CGlass keeps
only construction DTOs and derived native-slot/Entity membership, never owned
live pane matrices or active booleans. Native 45-slot allocation/source order
is independent of Entity order. Unguarded lifecycle maintenance consumes DTOs,
releases removals and retires complete actors on clear/reset, including pause
and loading. Late weapon-created panes materialize before presentation and
first advance in the next Glass stage. Dedicated glass submission reads the
same Components; the legacy street-effects path must not submit them again.
