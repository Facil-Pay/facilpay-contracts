# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Payment rolling reserve (#901)** — `set_rolling_reserve(admin, merchant, bps, hold_seconds)` withholds `bps` of each merchant settlement as a slice that `release_rolling_reserve(merchant)` pays out after `hold_seconds`. Held funds are visible through `get_rolling_reserve_held` and `get_rolling_reserve_slices`. Disabling the reserve (`bps = 0`) never releases held slices early.

- **Merge refund vouchers (#910)** — `merge_vouchers(customer, voucher_ids)` combines same-merchant, same-token, transferable, unexpired vouchers into one voucher with the earliest source expiry. Sources are marked consumed and `get_voucher_merged_into` points to the merged voucher.

- **Voucher liability cap (#914)** — The refund contract tracks outstanding voucher value per `(merchant, token)` (`get_voucher_liability`). `set_voucher_liability_cap` / `clear_voucher_liability_cap` cap it, and issuance beyond the cap fails with `VoucherLiabilityCapExceeded`. Redemption, the new `revoke_voucher` and the new `expire_voucher` reduce the liability.

- **One open refund request per payment (#915)** — Merchants can opt in with `set_single_open_request(merchant, true)`; `request_refund` then rejects a new request while the payment has one that is requested, pending appeal, or approved but unprocessed. Off by default.

- **Bounded emergency pause (#820)** — `emergency_pause_all` can set a maximum pause duration. Once it is reached, anyone may call `expire_emergency_pause()` to unpause all child contracts, which emits `EmergencyPauseAutoExpired`. Admin-set members can push the expiry back with `extend_emergency_pause(admin, additional_seconds)` before it is reached, and `get_emergency_pause_expiry()` returns the current expiry.

- **Refund contract in-place upgrades (#643)** — `upgrade(admin, new_wasm_hash)` replaces the refund contract's WASM without changing its address or storage, gated by the refund admin, and emits `ContractUpgraded`.

- **Merchant-initiated payment requests (#662)** — `create_payment_request`, `pay_payment_request`, `cancel_payment_request` and `get_payment_request` let merchants issue invoice links that a customer (optionally a specific one) pays exactly once, before expiry.

- **Refund admin rotation docs (#638)** — The refund README now documents `propose_admin`, `accept_admin` and `get_pending_admin`.

- **Payment Contract Events Documentation** — Comprehensive event reference for all 50+ Soroban events emitted by the payment contract, including core payments, subscriptions, channels, fees, governance, and control events. Off-chain integrators can now use this table to subscribe to events via Horizon.

- **Refund Contract Events Documentation** — Comprehensive event reference for all 20+ Soroban events emitted by the refund contract, including refund lifecycle, appeals, arbitration, and stake management events. Enables off-chain monitoring of refund status changes and arbitration outcomes.

- **CHANGELOG.md** — This file. Tracks all breaking changes, new features, and bug fixes across contract releases to help API/SDK consumers plan upgrades.

- **SECURITY.md** — Vulnerability disclosure policy and security contact information for responsible security research.

### Changed

- **Admin `emergency_pause_all` signature (Breaking, #820)** — Now `emergency_pause_all(pauser, reason, max_pause_seconds: Option<u64>)`. Pass `None` to keep the previous, unbounded behaviour.

- **Payment storage tiers (Breaking, #648)** — Per-record and per-user payment contract data moved from instance to persistent storage, with TTL extended on read and write. Schema version is now `2`. v1 deployments holding data are not migrated in place; see [docs/STORAGE_VERSIONING.md](docs/STORAGE_VERSIONING.md).

- **Refund Reason Code Migration (Breaking)** — The `request_refund()` function signature has changed to require a canonical `RefundReasonCode` enum variant in addition to free-text reason.
  - **Old signature:** `request_refund(..., reason: String, payment_created_at: u64)`
  - **New signature:** `request_refund(..., reason: String, reason_code: RefundReasonCode, payment_created_at: u64)`
  - **Reason:** Enables structured querying and analytics via `get_reason_code_analytics()` without free-form string inconsistency.
  - **Migration path:**
    1. Update all callers to pass a concrete enum value: `ProductDefect`, `NonDelivery`, `DuplicateCharge`, `Unauthorized`, `CustomerRequest`, or `Other`.
    2. For unknown/legacy flows, pass `Other` as a fallback and backfill specific codes in your upstream app logic.
    3. If upgrading a deployed instance with existing data, plan a storage/data migration for historical refunds before reading them as the new `Refund` shape.

### Fixed

- **Workspace build restored** — Recent merges had dropped storage-key variants (`TrustBoostConfig` in refund; `Hashlock`, `Note`, `NoteCount` in escrow) and the admin contract's `cancelled` proposal field and child-upgrade types, so `admin`, `escrow` and `refund` no longer compiled. The escrow keys now live in a new `EscrowExtKey` enum because `EscrowKey` is at Soroban's 50-case limit. Admin tests were updated for the `propose_action` description-hash argument.

- Restored enum variants and types dropped by earlier merges so the escrow, refund and payment contracts compile again; renumbered the refund counter-offer/voucher errors to `72`–`76` and the payment `PauseLimitExceeded` error to `320` to remove code collisions.

- Improved documentation coverage to reduce friction for off-chain integrators consuming Soroban events.

- **Split Payment Dust Recipient Guard** — `create_split_payment()` rejects any recipient whose computed share falls below an admin-configured `min_split_amount` floor, preventing dust splits from bloating ledger storage. Enforced via `set_min_split_amount()` / `get_min_split_amount()` (see commit `061aeeb`).

---

## [Previous Versions]

For detailed information on previous releases, see the [Root README](README.md) and individual contract READMEs in the `contracts/` directory.

---

## Guidelines for Contributors

When proposing changes that affect consumers of these contracts:

1. **Breaking changes** — Document the change here under `[Unreleased]` → `### Changed`, include migration guidance, and consider the impact on API/SDK versions.
2. **New features** — Add under `### Added` with a brief description of the feature and where it's used.
3. **Bug fixes** — Add under `### Fixed` with a reference to the issue (if applicable).
4. **Dependencies** — Note any upgrades to Soroban SDK or Rust toolchain under `### Changed`.

Semantic versioning:
- **MAJOR** — breaking changes to contract interfaces (e.g., function signature changes, new required parameters)
- **MINOR** — additive features that don't break existing code
- **PATCH** — bug fixes and internal improvements
