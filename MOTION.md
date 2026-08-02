---
schema: design-pipeline.motion-foundation.v0.1
name: Mark Time motion language
posture: static
primitiveRegistry: design-pipeline.motion-primitives.v1
---

## Motion Thesis

Mark Time reports time-sensitive claims. Motion may never imply that a rule changed before the
service supplied a new snapshot. Time advances as updated data, not decoration.

## Motion Principles

- Prefer immediate, stable replacement of a server snapshot over animated row rearrangement.
- The one-second clock update changes numerals, local clocks, and the same UTC cursor position
  only; it never moves layout or alters a server timeline segment.
- Focus and pressed feedback are brief CSS state changes, never required to understand a result.

## Motion Vocabulary

No registry primitive is selected. The product has no scene, procedural path, or choreography.

## Procedural Motion

None. The timetable cursor is a data position derived from the displayed UTC instant, not a generated
animation. It remains stationary in inspection mode.

## Runtime Policy

Native CSS may transition color, border-color, and background-color for controls in 120–160ms.
D3 renders a static SVG world underlay once; it owns no render loop,
transition, interpolation, or animation. DOM content updates preserve user focus and use
`aria-live` only for concise snapshot summaries.

## Reduced Motion

When `prefers-reduced-motion: reduce` is active, controls change immediately and no transform or
opacity transition is emitted. The clock and horizon remain readable because they are data updates,
not motion cues. Fallback: static final control state with the same text, focus, and contrast.

## Source Decisions

This is an authored static policy for `market-time-clockboard-design`.

- Adopted: the accessibility requirement to provide a reduced-motion substitution and the product
  rule that time advances as data, not visual decoration.
- Rejected: decorative ambient, animated map, particles, cursor-following effects, and animated
  phase bars; their product value is lower than their potential to imply a rule change.
- Code copied: none.
