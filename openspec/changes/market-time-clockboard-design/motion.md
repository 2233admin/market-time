# Motion

Foundation: [`MOTION.md`](../../../MOTION.md)

- Foundation SHA-256: `9f8fa47493b93ff11767f09a63f4283ad4cd039a31970dbc5908cdf42755fe76`
- Posture: `static`
- Selected primitive IDs: none
- Runtime: native DOM data updates and bounded CSS control feedback; D3 renders a static SVG world
  underlay without a render loop or animation.

## Binding

The UTC and local-clock numerals update as data. A market-horizon cursor may change position only
when the displayed reference instant changes; in inspection mode it is static. The current phase
bar never animates or interpolates—it is redrawn only from the service snapshot. Snapshot
replacement is immediate, retains keyboard focus, and never reorders readings by animation.
Control hover/focus states may transition colour or border colour in 120–160ms and must be gated
behind appropriate pointer media queries.

## Reduced Motion

For `prefers-reduced-motion: reduce`, controls have their final state immediately. No transform,
opacity, or position transition is required to discover a status change. This is the selected
fallback from the project foundation.
