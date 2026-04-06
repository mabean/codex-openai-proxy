# Codex OpenAI Proxy Hardening Plan

Date: 2026-04-06
Status: planned
Owner: Штирлиц

## Goal
Turn `codex-openai-proxy` from a partially stubbed prototype into a minimal, auditable, local-only proxy that can be evaluated as a realistic basis for Codex OAuth → OpenAI-compatible and Anthropic-compatible bridging.

---

## Guiding principles

1. **Local-first and minimal**
   - No cloud relay
   - No unnecessary dashboard/UI features
   - No hidden side effects

2. **Transparent behavior**
   - No fake success responses when upstream fails
   - Clear health and error reporting
   - Easy to inspect request/response flow

3. **Token safety first**
   - Read local auth only
   - Do not log secrets
   - Bind to localhost by default
   - Avoid exposing via ngrok by default in docs

4. **Small attack surface**
   - Only implement needed endpoints
   - Remove speculative headers/fields unless justified
   - Prefer deterministic transformations over broad emulation

5. **Auditable and reusable by design**
   - Keep the trusted codebase small
   - Pin and review dependencies
   - Make builds reproducible
   - Document threat model and security guarantees clearly

---

# Work packages

## WP1 — Reality alignment / documentation cleanup

### Problems
- README currently overclaims maturity and features.
- README suggests usage patterns (e.g. ngrok) that are not appropriate as a secure default.
- Current implementation returns a development stub while docs imply a real backend path.

### Actions
- Rewrite README to match actual state.
- Remove or demote claims that are not yet true.
- Mark security-sensitive deployment patterns (public tunnel / remote exposure) as unsafe-by-default.
- Add an explicit architecture section:
  - OpenAI-compatible ingress
  - Codex backend egress
  - local auth file usage
  - local-only trust boundary
- Add a clear “current status” section.

### Deliverable
- Clean, honest `README.md`

---

## WP2 — Remove the stub and promote the real execution path

### Problems
- `proxy_request(...)` currently returns a fake canned response.
- The real implementation lives in `proxy_request_original(...)`, disconnected from the main path.

### Actions
- Delete the fake response path.
- Rename the real path to become the main execution path.
- Ensure the actual request path is the default runtime behavior.
- Keep a debug/mock mode only if explicitly gated and clearly named.

### Deliverable
- No hidden dev stub in main request path
- Upstream failures surface as real errors instead of fake assistant success

---

## WP3 — Auth model hardening

### Problems
- Current auth structure is too opinionated and may not match real OpenClaw/OpenAI auth stores.
- Refresh handling is unclear/incomplete.
- Auth path defaults are too narrow.

### Actions
- Define explicit auth source strategy:
  1. local auth.json path
  2. explicit env override
  3. future: pluggable auth readers
- Align auth parsing with the real credential store shape we actually use.
- Add validation on startup:
  - token present
  - account_id present when required
  - clear error if auth file shape is unsupported
- Do not log raw access tokens, refresh tokens, or account identifiers.

### Deliverable
- Explicit, validated auth loading path

---

## WP4 — Transport contract cleanup

### Problems
- Request currently includes a large set of browser-like headers, some likely cargo-culted.
- It is unclear which headers are actually required.
- Contract should be aligned with the observed Codex/OpenClaw path rather than speculative browser emulation.

### Actions
- Reduce headers to a minimal known-good set.
- Classify headers into:
  - required
  - optional/observed
  - remove
- Start from observed Codex/OpenClaw contract:
  - `Authorization`
  - `chatgpt-account-id`
  - `OpenAI-Beta`
  - `session_id` (if required by tested path)
  - `originator` / user-agent only if proven needed
- Remove browser-fingerprint noise unless required by real behavior.
- Separate transport-specific config from request transformation logic.

### Deliverable
- Minimal, documented upstream request contract

---

## WP5 — Request transformation cleanup

### Problems
- Chat Completions → Codex Responses transformation is plausible but should be stricter and easier to reason about.
- Current transformation may carry fields we don’t need.
- Future Anthropic-compatible ingress should not be bolted on as an afterthought.

### Actions
- Document the transformation contract explicitly.
- Normalize message content deterministically.
- Keep only fields required for supported use cases.
- Reject unsupported constructs clearly instead of silently inventing behavior.
- Decide exact supported ingress surface for v1/v2:
  - `/v1/chat/completions`
  - optional `/v1/responses` later
  - future `/v1/messages` (Anthropic-compatible) after OpenAI-compatible path is stable
- Design the internal representation so both OpenAI-style and Anthropic-style requests can map into the same Codex backend request model.

### Deliverable
- Minimal, explicit request transformation layer
- Clear path for future Anthropic-compatible ingress

---

## WP6 — Response and streaming correctness

### Problems
- Real backend response handling needs to be the primary path.
- Streaming claims must match real behavior.
- Error translation needs to be explicit.

### Actions
- Make non-streaming path correct first.
- Then verify streaming path separately.
- Clearly map upstream errors to downstream responses.
- Do not mask upstream failures as successful assistant replies.
- Add structured response parsing tests.

### Deliverable
- Correct non-streaming path
- Streaming path either verified or explicitly marked unsupported for current version

---

## WP7 — Error handling and observability

### Problems
- Current path can hide backend failures.
- Debugging production issues would be difficult.

### Actions
- Add clear error classes/log messages for:
  - auth missing
  - auth invalid
  - unauthorized
  - rate-limited
  - upstream schema mismatch
  - unsupported feature
- Health endpoint should reflect auth readiness and backend readiness separately.
- Add safe debug logging with secret redaction.

### Deliverable
- Honest operational behavior

---

## WP8 — Security hardening defaults

### Problems
- README encourages patterns that broaden exposure too early.
- Proxy security boundary is underdefined.

### Actions
- Bind to `127.0.0.1` by default.
- Document public exposure as advanced/unsafe-by-default.
- Add optional API key / shared secret for local clients if needed.
- Consider allowlist of origins / loopback-only mode.
- Explicitly warn against copying auth files around casually.

### Deliverable
- Safe default runtime posture

---

## WP9 — Tests and verification

### Problems
- Current maturity claims are not yet backed by strong verification.

### Actions
- Add unit tests for:
  - auth parsing
  - request conversion
  - response parsing
  - error translation
- Add mock-based upstream tests.
- Add one end-to-end local smoke test path.
- Add a checklist for live validation with a real token.

### Deliverable
- Credible test baseline

---

## WP10 — Supply-chain and build trust

### Problems
- A reusable fork needs stronger guarantees than "the code looks okay".
- Dependency and release hygiene are currently undocumented.

### Actions
- Commit and maintain `Cargo.lock`.
- Prefer pinned/minimal dependencies.
- Consider `cargo vendor` for high-trust/offline builds.
- Add `BUILD.md` with reproducible build steps.
- Add CI steps for:
  - `cargo build --frozen`
  - `cargo test --frozen`
  - `cargo clippy -- -D warnings`
  - secret/logging grep checks
- Add release verification guidance (checksums, optional signed tags).

### Deliverable
- Reproducible and reviewable build process

---

## WP11 — Threat model and security policy

### Problems
- Security expectations are currently implicit.
- There is no explicit statement of trust boundaries and non-goals.

### Actions
- Add `SECURITY.md`.
- Add `THREAT_MODEL.md`.
- Document:
  - allowed outbound hosts
  - secret-handling rules
  - localhost-only default trust boundary
  - unsupported/unsafe deployment patterns
  - no-telemetry/no-cloud-relay policy
- Add an auditable security checklist for reviewers.

### Deliverable
- Explicit security posture and review checklist

---

## WP12 — Dual-API surface planning (OpenAI + Anthropic)

### Problems
- We want the proxy to remain small, but future Anthropic-compatible ingress should be planned early.
- If not planned now, the second API surface may force awkward rewrites later.

### Actions
- Define one internal canonical request/response model.
- Keep auth and transport shared underneath both API surfaces.
- Explicitly scope Anthropic-compatible ingress as a later workstream built on the hardened OpenAI-compatible core.
- Add dedicated notes/doc for Anthropic-compatible support.

### Deliverable
- A clean architecture path for both OpenAI-compatible and Anthropic-compatible gateway behavior

---

## WP13 — README security and trust communication

### Problems
- Security-sensitive projects need visible trust signals.
- Users should not have to read source code first to understand the risk posture.

### Actions
- Add a top-level README security section.
- Add a status panel/badge block such as:
  - local-only by default
  - no cloud relay
  - secrets redacted in logs
  - lockfile committed
  - tests green
  - security docs present
- Add a direct link to `SECURITY.md`, `THREAT_MODEL.md`, and `BUILD.md`.
- Clearly separate:
  - current guarantees
  - planned hardening work
  - unsafe/advanced deployment modes

### Deliverable
- README communicates trust boundaries and current hardening state clearly

---

## WP14 — Decision point

After WP1–WP9, decide one of:
1. Use this fork as the basis for dashboard integration
2. Vendor only selected parts/patterns into a smaller internal proxy
3. Abandon this fork and write a thinner proxy informed by what was learned

### Decision criteria
- Token safety
- Code simplicity
- Ease of auditing
- Ease of maintenance
- Match to dashboard needs

---

# Recommended execution order
1. WP1 README cleanup
2. WP2 remove stub / activate real path
3. WP3 auth hardening
4. WP4 transport cleanup
5. WP5 request transformation cleanup
6. WP6 response correctness
7. WP7 observability
8. WP8 security defaults
9. WP9 tests
10. WP10 supply-chain and build trust
11. WP11 threat model and security policy
12. WP12 dual-API surface planning
13. WP13 README security and trust communication
14. WP14 decision

---

# Definition of done
- README is honest and includes visible security/trust posture
- No fake success path remains
- Local-only secure default works
- Auth loading is explicit and validated
- Request/response behavior is documented and tested
- Real upstream failures are visible, not hidden
- Dependency/build process is reproducible and reviewable
- Security docs and threat model exist
- Architecture keeps a clean path for both OpenAI-compatible and Anthropic-compatible ingress
- We can confidently decide whether to adopt/fork/rewrite
