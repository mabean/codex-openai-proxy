# Anthropic-Compatible Gateway Notes

Date: 2026-04-06
Status: planned

## Why this matters
Some tools expect Anthropic-style endpoints (for example `/v1/messages`) rather than OpenAI-style endpoints.
If the proxy becomes a clean Codex OAuth bridge, it is strategically useful to support both:
- OpenAI-compatible ingress
- Anthropic-compatible ingress

## Goal
Expose an Anthropic-like surface on top of the same local Codex-backed core, without duplicating transport or auth logic.

## Design direction
- Keep one internal canonical request model
- Build thin ingress adapters:
  - OpenAI Chat Completions / Responses
  - Anthropic Messages
- Reuse one Codex transport layer
- Keep response/error translation explicit per API family

## Constraints
- Anthropic-compatible support should come after the OpenAI-compatible path is correct and hardened
- No extra remote services
- Same token safety and localhost-only defaults must apply

## Likely future work
1. Define supported Anthropic subset for v1
2. Map Anthropic messages → internal model
3. Map internal result → Anthropic response shape
4. Add Anthropic streaming/error translation
5. Add dedicated tests and docs
