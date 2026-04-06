# SECURITY

## Security posture
This project is intended to become a **local-only proxy** for Codex/ChatGPT OAuth-backed access with an OpenAI-compatible ingress surface.

Current security goals:
- localhost-only bind by default
- no cloud relay
- no hidden telemetry
- no secret logging
- minimal outbound host set
- reproducible builds and pinned dependencies

## Sensitive assets
- OAuth access tokens
- account identifiers
- local auth files
- downstream prompts/responses

## Rules
- Never log raw bearer tokens or auth files
- Do not expose the proxy publicly by default
- Do not commit local auth files
- Treat reverse tunnels/public bind as advanced and unsafe until separately hardened

## Allowed outbound hosts (target policy)
- `chatgpt.com`
- auth-related OpenAI hosts only if explicitly needed by future refresh logic

## Reporting
If a change broadens token exposure, adds telemetry, or introduces new outbound destinations, it should be treated as security-relevant and called out explicitly in review.
