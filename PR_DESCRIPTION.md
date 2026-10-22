# Pull Request: Escrow Lifecycle Hooks & Query Pagination Documentation

## Summary

This PR documents the previously undocumented public escrow functions for
lifecycle hook registration and paginated escrow queries, resolving issue #826.

No contract logic is changed — this is a documentation-only update to
`contracts/escrow/README.md`.

## Changes

### `contracts/escrow/README.md`

**New section — Escrow Lifecycle Hooks**

Documents the publish-subscribe notification system:
- `register_escrow_hook` — subscriber registration with `ping()` validation,
  per-event cap (10), auth requirements, and returned `hook_id`.
- `deregister_escrow_hook` — marks a hook inactive; only the original
  subscriber may deregister.
- `get_escrow_hooks` — lists all active hooks for a given event type.
- Full `EscrowEventType` table (`Created`, `Released`, `Disputed`, `Resolved`,
  `Refunded`, `Cancelled`, `StatusChanged`).
- Subscriber interface contract (`ping()` + `on_escrow_event()`).
- Error codes, emitted events, and a usage example.

**New section — Escrow Queries and Pagination**

Documents all six paginated query functions with clear offset/limit semantics:
- `get_escrows_by_status` / `get_escrow_count_by_status`
- `get_escrows_by_customer` / `get_escrow_count_by_customer`
- `get_escrows_by_merchant` / `get_escrow_count_by_merchant`
- Pagination examples (full-scan loop, single-page fetch, count-then-page).
- Implementation note on the append-only status index and stale-entry skipping.
- Note on `total_volume` being a raw cross-token sum.

## Testing

Documentation-only change. No new code paths are introduced and no existing
tests are modified.

## Closes

Closes #759, closes #760, closes #826, closes #762
