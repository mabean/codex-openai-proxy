# Anthropic-Compatible Gateway Notes

Date: 2026-04-06
Status: baseline implemented

## Why this matters
Some tools expect Anthropic-style endpoints (for example `/v1/messages`) rather than OpenAI-style endpoints.
If the proxy becomes a clean Codex OAuth bridge, it is strategically useful to support both:
- OpenAI-compatible ingress
- Anthropic-compatible ingress

## Current baseline
Implemented:
- `POST /v1/messages`
- Anthropic request → internal chat conversion
- Internal chat result → Anthropic message response shape
- Structured Anthropic error envelopes
- Request validation for key Anthropic fields
- Shared upstream transport and SSE text extraction logic

Current constraints:
- non-streaming only
- text-only baseline
- usage accounting is placeholder/zeroed
- no tool-use or multimodal Anthropic surface yet

## Design direction
- Keep one internal canonical request model
- Build thin ingress adapters:
  - OpenAI Chat Completions / Responses
  - Anthropic Messages
- Reuse one Codex transport layer
- Keep response/error translation explicit per API family

## Error translation baseline
Anthropic responses now return structured envelopes like:
- `invalid_request_error`
- `not_implemented_error`
- `authentication_error`
- `api_error`

Validation is explicit for:
- missing `model`
- empty `messages`
- missing/zero `max_tokens`
- invalid Anthropic message roles

## Remaining future work
1. Add Anthropic streaming support
2. Support richer Anthropic content blocks where useful
3. Improve token/usage accounting when upstream contract is clearer
4. Add HTTP-level integration tests for `/v1/messages`
5. Document supported Anthropic subset in README
