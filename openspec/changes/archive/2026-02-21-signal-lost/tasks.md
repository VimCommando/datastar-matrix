## 1. Browser Signal Lost Rendering

- [x] 1.1 Remove separate disconnect overlay usage from browser markup and keep a single canvas rendering surface
- [x] 1.2 Add in-band `[ SIGNAL LOST ]` canvas rendering that centers the message in matrix cell coordinates
- [x] 1.3 Keep stale timeout behavior and trigger Signal Lost rendering when stream updates are stale

## 2. Recovery and Frame Coherence

- [x] 2.1 Clear Signal Lost state on first fresh frame and resume normal matrix rendering automatically
- [x] 2.2 Preserve existing stale-frame guard so out-of-order frames are ignored during normal rendering
- [x] 2.3 Keep resize behavior coherent by re-rendering Signal Lost treatment while disconnected

## 3. Verification

- [x] 3.1 Add/maintain browser markup assertions for Signal Lost text and disconnect logic in web tests
- [x] 3.2 Validate secure-origin `/events` behavior remains functional with HTTPS test coverage
