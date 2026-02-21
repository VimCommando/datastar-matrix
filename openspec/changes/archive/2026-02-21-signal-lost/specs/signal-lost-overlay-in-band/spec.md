## ADDED Requirements

### Requirement: In-band Signal Lost matrix rendering
The browser renderer MUST display a Signal Lost indicator as matrix characters embedded in the canvas output rather than as a separate HTML overlay element.

#### Scenario: Stream becomes stale
- **WHEN** frame updates stop and stale timeout is reached
- **THEN** the renderer draws an ASCII-style `[ SIGNAL LOST ]` message centered within the matrix character grid

#### Scenario: Signal Lost indicator style remains matrix-native
- **WHEN** the Signal Lost treatment is rendered
- **THEN** message glyphs are composed through the matrix canvas rendering path and use matrix character styling semantics

### Requirement: Signal Lost recovery behavior
The browser renderer MUST remove the in-band Signal Lost treatment once fresh stream frames resume.

#### Scenario: Stream updates resume after stale state
- **WHEN** a new valid frame is received after Signal Lost rendering was active
- **THEN** normal matrix frame rendering resumes without requiring page reload
