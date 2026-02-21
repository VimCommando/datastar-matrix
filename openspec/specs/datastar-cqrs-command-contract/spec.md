## ADDED Requirements

### Requirement: Command ingress uses REST acknowledgement without state payload
The system SHALL accept browser-issued commands via HTTP `POST` endpoints under `/cmd/*` and MUST acknowledge accepted commands with `204 No Content` without returning rendered state payloads.

#### Scenario: Browser sends a valid command
- **WHEN** the client posts a supported command to `/cmd/{op}`
- **THEN** the server responds with `204 No Content`
- **AND** the response body is empty

#### Scenario: Browser sends an unsupported command operation
- **WHEN** the client posts to `/cmd/{op}` with an unknown operation
- **THEN** the server response does not include frame-state payload

### Requirement: Query and UI state remain authoritative on SSE
The browser integration MUST treat `/events` SSE frames and Datastar signal patches as the authoritative source for rendered matrix state and SHALL NOT require command response bodies to update UI state.

#### Scenario: Command is acknowledged before next frame
- **WHEN** a command request receives `204` and no body
- **THEN** the client continues rendering based on subsequent `/events` stream updates

#### Scenario: Multiple commands are sent in quick succession
- **WHEN** the client issues sequential command posts
- **THEN** rendered state convergence is determined by the order and content of subsequent SSE frame updates
