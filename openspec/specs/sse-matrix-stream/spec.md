## Purpose

Define server-side streaming behavior for matrix updates, including endpoint contract, synchronization behavior, concurrency handling, and telemetry expectations.

## Requirements

### Requirement: SSE endpoint for matrix updates
The server SHALL expose an HTTP Server-Sent Events endpoint at `/events` that publishes matrix frame updates for connected clients.

#### Scenario: Client connects to matrix stream endpoint
- **WHEN** a client performs a GET request to the configured matrix events endpoint
- **THEN** the server responds with `text/event-stream` and begins emitting frame update events

### Requirement: CLI port selection with random default
The application SHALL expose `--port` as a command-line option for web server binding, and MUST choose a random available port when `--port` is not provided.

#### Scenario: User omits port flag
- **WHEN** the application starts with web support enabled and no `--port` argument
- **THEN** the server binds to a random available port and starts serving the SSE endpoint

#### Scenario: User sets explicit port
- **WHEN** the application starts with `--port <value>`
- **THEN** the server attempts to bind to the requested port

### Requirement: Frame payload transport contract
Each SSE event MUST include a structured payload sufficient for clients to reconstruct the current matrix frame state, using sparse delta updates for normal ticks and full keyframes for synchronization points.

#### Scenario: Event is emitted for a simulation tick
- **WHEN** the simulation produces a new frame
- **THEN** the corresponding SSE event includes frame identifier and matrix state fields required by consumers

### Requirement: Late-join synchronization
The server SHALL deliver a full keyframe to newly connected SSE clients on the next simulation tick, after which it SHALL deliver sparse delta updates.

#### Scenario: Browser client connects mid-stream
- **WHEN** a new client subscribes while the matrix stream is already running
- **THEN** the first frame event delivered to that client on the next tick is a full keyframe and subsequent events are sparse deltas

### Requirement: Multi-client stream handling
The stream transport SHALL support multiple concurrent SSE clients without stopping terminal rendering or simulation progression.

#### Scenario: Additional browser client subscribes while stream is active
- **WHEN** a second client connects to the SSE endpoint
- **THEN** both clients receive ongoing frame events while simulation remains active

### Requirement: Slow-client backpressure policy
The stream transport MUST prioritize freshness by dropping older queued updates for slow clients rather than blocking simulation or terminal rendering.

#### Scenario: Client cannot keep pace with event rate
- **WHEN** a client falls behind the active stream rate
- **THEN** the server drops stale queued frames for that client and continues delivering newer frames

### Requirement: Compile-time web server feature gate
Web server and SSE functionality MUST be disabled when the web feature flag is not enabled at compile time.

#### Scenario: Application built without web feature
- **WHEN** the binary is compiled without the web server feature flag
- **THEN** web server startup and SSE endpoints are not available at runtime

### Requirement: Coupled runtime failure shutdown
When both terminal and web components are enabled, failure in either component MUST trigger coordinated shutdown of the full application.

#### Scenario: Web server fails during active runtime
- **WHEN** the web server encounters a fatal runtime error while terminal rendering is active
- **THEN** the application initiates shutdown and terminal rendering stops

#### Scenario: Terminal loop fails during active runtime
- **WHEN** the terminal rendering loop encounters a fatal runtime error while web server is active
- **THEN** the application initiates shutdown and web server stops

### Requirement: Stream telemetry counters
The runtime MUST maintain stream telemetry counters for active `clients`, emitted `frames`, and dropped updates (`drops`) for use in terminal observability.

#### Scenario: Stream activity changes over time
- **WHEN** clients connect or disconnect and frames are emitted or dropped
- **THEN** telemetry counters are updated to reflect current clients and cumulative frames and drops

### Requirement: Unauthenticated HTTP access for current stage
For this stage, web and SSE endpoints SHALL accept unauthenticated HTTP connections with no application-level authentication requirement.

#### Scenario: Client connects without credentials
- **WHEN** a client requests web or SSE endpoints without authentication headers or tokens
- **THEN** the server allows the connection if networking and feature conditions are otherwise valid
