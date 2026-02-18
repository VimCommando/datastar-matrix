## ADDED Requirements

### Requirement: Shared matrix state timeline
The system SHALL maintain a single simulation timeline for matrix columns and glyph states that is consumed by all outputs.

#### Scenario: Two consumers subscribe to the same simulation
- **WHEN** terminal rendering and web streaming are active at the same time
- **THEN** both consumers receive updates derived from the same simulation tick sequence

### Requirement: Non-deterministic run initialization
The simulation MUST initialize with fresh randomness for each application run so starting matrix state differs across runs.

#### Scenario: Application is launched multiple times
- **WHEN** the application is started in separate runs with default settings
- **THEN** initial matrix state is not required to match between runs

### Requirement: Ordered frame progression contract
The simulation MUST advance state in discrete ticks with a monotonically increasing frame identifier within a single run.

#### Scenario: Consecutive frames are generated during one run
- **WHEN** the simulation advances across multiple ticks
- **THEN** each emitted frame has a frame identifier greater than the previous frame identifier

### Requirement: Configurable simulation parameters
The system SHALL support configurable simulation parameters including dimensions, tick rate, and glyph source set.

#### Scenario: Non-default simulation config is provided
- **WHEN** the application starts with explicit simulation settings
- **THEN** generated frames reflect the configured dimensions, cadence, and allowed glyph set

### Requirement: Default glyph class weighting
The simulation MUST default glyph selection weights to 60% numeric, 20% alphabetic, and 20% katakana.

#### Scenario: Glyph generation uses default weights
- **WHEN** glyphs are selected without overriding class weights
- **THEN** numeric glyphs are sampled with 60% weight, alphabetic glyphs with 20% weight, and katakana glyphs with 20% weight

### Requirement: Tunable glyph class weighting constants
The system SHALL define glyph class weights as configurable constants so weights can be adjusted without changing core generation flow.

#### Scenario: Weight constants are updated
- **WHEN** configured glyph class constants are changed
- **THEN** subsequent generated glyphs reflect the updated class weighting

### Requirement: Default target framerate
The simulation MUST default to a target framerate of 60 FPS when no explicit framerate configuration is provided.

#### Scenario: Application starts with default framerate settings
- **WHEN** the user launches the application without specifying a target framerate
- **THEN** the simulation and update cadence target 60 FPS

### Requirement: CLI framerate override
The application SHALL expose `--fps` as a command-line option to set target simulation framerate.

#### Scenario: User provides fps flag
- **WHEN** the user launches the application with `--fps <value>`
- **THEN** the simulation targets the provided framerate value

### Requirement: Shared clock authority
The simulation clock MUST be the authoritative cadence source for both terminal rendering and SSE frame publication.

#### Scenario: Terminal and SSE outputs run concurrently
- **WHEN** the application runs with both output paths enabled
- **THEN** terminal and SSE updates are emitted from the same simulation tick sequence

### Requirement: Resize state continuity
The simulation SHALL apply top-left anchoring semantics on dimension changes: clipping state on shrink and extending state space on growth.

#### Scenario: Simulation dimensions decrease
- **WHEN** target simulation width or height is reduced
- **THEN** state outside the new bounds is clipped and in-bounds state remains top-left aligned

#### Scenario: Simulation dimensions increase
- **WHEN** target simulation width or height is increased
- **THEN** new rows and columns are added and become eligible for normal top-to-bottom character generation in subsequent ticks
