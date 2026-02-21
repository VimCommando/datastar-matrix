## Context

The matrix web client currently mixes Datastar attributes with a large imperative JavaScript controller. Operationally, the system already follows CQRS: browser commands are posted to `/cmd/*` and the backend returns `204`, while frame/state updates flow through `/events` as SSE and are mapped into Datastar signals. The mismatch is mostly contractual and stylistic: specs do not explicitly define this split, and frontend interaction patterns do not clearly prioritize `data-*` attributes except for stream bootstrap.

## Goals / Non-Goals

**Goals:**
- Preserve the intentional CQRS transport split (`POST /cmd/*` for commands, `/events` SSE for query/state).
- Make the contract explicit in specs so audits and implementations stop treating `204` command responses as a defect.
- Increase Datastar-first frontend patterns by moving command dispatch triggers and signal transitions into `data-*` attributes where practical.
- Keep canvas rendering performant and stable while minimizing non-essential vanilla JavaScript.

**Non-Goals:**
- Replacing canvas rendering with DOM-based rendering.
- Eliminating all custom JavaScript.
- Introducing a new API surface beyond existing command and SSE endpoints.
- Changing simulation cadence, ripple behavior, or TLS behavior.

## Decisions

1. Define an explicit CQRS capability for command/query transport.
- Decision: add a new `datastar-cqrs-command-contract` spec with requirements for `POST /cmd/*` `204` acknowledgements and SSE as the authoritative state channel.
- Rationale: removes ambiguity and aligns audits with intentional architecture.
- Alternatives considered:
  - Fold this entirely into existing specs. Rejected because CQRS semantics span multiple concerns and deserve a single normative contract.

2. Modify `datastar-matrix-sync` to prefer `data-*` event bindings for command initiation.
- Decision: specify that keyboard, resize, and pointer event wiring should originate from Datastar attributes, with JS limited to rendering/math helpers.
- Rationale: keeps interaction declarative while preserving required imperative canvas code.
- Alternatives considered:
  - Keep all event routing in global JS. Rejected because it obscures signal/event flow and dilutes Datastar-first conventions.

3. Clarify stream contract independence from command acknowledgements.
- Decision: modify `sse-matrix-stream` to assert that command `204` responses never carry rendered state; clients must observe authoritative state via SSE.
- Rationale: enforces single source of truth for UI state and avoids accidental coupling to command response payloads.
- Alternatives considered:
  - Return partial state in command responses. Rejected because it weakens CQRS separation and duplicates state channels.

4. Keep a minimal JS compatibility layer for browser/canvas constraints.
- Decision: explicitly allow a narrow helper layer for canvas painting, coordinate conversion, and browser API interactions not expressible in Datastar attributes.
- Rationale: preserves rendering behavior while still reducing non-essential imperative code.
- Alternatives considered:
  - Force full declarative-only frontend. Rejected because required browser APIs and canvas rendering remain imperative.

## Risks / Trade-offs

- [Hybrid declarative/imperative boundary may still be debated] -> Mitigation: codify minimal imperative carve-out (canvas paint + coordinate transforms only) in specs and tasks.
- [Refactoring event wiring could regress command behavior] -> Mitigation: add tests for key bindings, click-to-glitch mapping, and resize command dispatch expectations.
- [Developers may infer `data-*` means no REST commands] -> Mitigation: explicitly specify that Datastar attributes may trigger REST command posts under CQRS.
