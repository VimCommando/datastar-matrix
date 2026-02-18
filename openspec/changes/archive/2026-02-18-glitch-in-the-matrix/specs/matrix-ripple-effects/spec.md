## ADDED Requirements

### Requirement: Click-triggered glitch ripple creation
The system SHALL create a glitch ripple when it receives `POST /cmd/glitch` with JSON payload `{"x": int, "y": int}` that identifies a matrix cell coordinate.

#### Scenario: Valid glitch command creates ripple
- **WHEN** the backend receives `POST /cmd/glitch` with in-bounds integer `x` and `y`
- **THEN** a new ripple is registered with origin at that cell for simulation on subsequent ticks

#### Scenario: Multiple glitch commands arrive in one frame window
- **WHEN** more than one valid glitch command arrives before the next simulation frame is produced
- **THEN** at most one new ripple is created for that frame window

### Requirement: Ripple radius and duration constraints
Each ripple MUST expand outward as a circular wave with a maximum reach of 16 character cells and MUST fully expire after 750ms.

#### Scenario: Ripple reaches configured maximum radius
- **WHEN** ripple propagation is computed over time
- **THEN** influence does not extend beyond 16 cells from the ripple origin

#### Scenario: Ripple reaches end of lifetime
- **WHEN** 750ms have elapsed since ripple creation
- **THEN** the ripple is removed and no longer affects frame output

#### Scenario: Leading edge bright hold before fade
- **WHEN** a ripple is in its first ~400ms of lifetime
- **THEN** the leading edge luminance remains full-bright before decay begins

### Requirement: Ripple alters glyph and luminance of existing cells
Active ripples MUST perturb both glyph selection and luminance values of existing matrix cells, with influence fading over the ripple lifetime.

#### Scenario: Cell is near active wavefront
- **WHEN** a cell falls within active ripple influence
- **THEN** the cell output includes both glyph and luminance modification relative to baseline rain simulation

#### Scenario: Ripple influence decays over time
- **WHEN** ripple age increases toward expiration
- **THEN** glyph and luminance perturbation strength decreases until no effect remains

### Requirement: Shared simulation parity for ripple effects
Ripple effects SHALL be computed in shared simulation state so terminal and browser outputs represent the same ripple-influenced frame state.

#### Scenario: Terminal and browser render same active ripple frame
- **WHEN** a frame is generated while a ripple is active
- **THEN** both terminal and browser renderers consume the same ripple-adjusted grid state for that frame identifier
