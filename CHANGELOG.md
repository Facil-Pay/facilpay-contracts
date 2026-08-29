# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- **Payment Contract Events Documentation** — Comprehensive event reference for all 50+ Soroban events emitted by the payment contract, including core payments, subscriptions, channels, fees, governance, and control events. Off-chain integrators can now use this table to subscribe to events via Horizon.

- **Refund Contract Events Documentation** — Comprehensive event reference for all 20+ Soroban events emitted by the refund contract, including refund lifecycle, appeals, arbitration, and stake management events. Enables off-chain monitoring of refund status changes and arbitration outcomes.

- **CHANGELOG.md** — This file. Tracks all breaking changes, new features, and bug fixes across contract releases to help API/SDK consumers plan upgrades.

- **SECURITY.md** — Vulnerability disclosure policy and security contact information for responsible security research.

### Changed

- **Refund Reason Code Migration (Breaking)** — The `request_refund()` function signature has changed to require a canonical `RefundReasonCode` enum variant in addition to free-text reason.
  - **Old signature:** `request_refund(..., reason: String, payment_created_at: u64)`
  - **New signature:** `request_refund(..., reason: String, reason_code: RefundReasonCode, payment_created_at: u64)`
  - **Reason:** Enables structured querying and analytics via `get_reason_code_analytics()` without free-form string inconsistency.
  - **Migration path:**
    1. Update all callers to pass a concrete enum value: `ProductDefect`, `NonDelivery`, `DuplicateCharge`, `Unauthorized`, `CustomerRequest`, or `Other`.
    2. For unknown/legacy flows, pass `Other` as a fallback and backfill specific codes in your upstream app logic.
    3. If upgrading a deployed instance with existing data, plan a storage/data migration for historical refunds before reading them as the new `Refund` shape.

### Fixed

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
