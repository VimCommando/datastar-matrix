## MODIFIED Requirements

### Requirement: Frame payload transport contract
Each SSE event MUST include a structured payload sufficient for clients to reconstruct the current matrix frame state, and the payload MUST include active ripple effects applied to glyph and luminance values while preserving existing frame cadence and pause heartbeat behavior.

#### Scenario: Event is emitted for a simulation tick
- **WHEN** the simulation produces a new frame
- **THEN** the corresponding SSE event includes frame identifier and matrix state fields required by consumers

#### Scenario: Ripple is active during frame emission
- **WHEN** one or more ripples are active during a simulation tick
- **THEN** emitted frame payload reflects ripple-adjusted glyph and luminance values for affected cells
