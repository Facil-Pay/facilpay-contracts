# Glossary

## Clawback

An admin-controlled mechanism that forces the return of escrowed funds to the admin, subject to a mandatory delay period (minimum 1 day). Clawback follows a three-phase lifecycle: initiate (with a reason hash), execute (after the delay elapses), and cancel.

## Escrow

A smart contract that locks funds between a customer and a merchant while a transaction is pending, disputed, or held. Escrows track status through a lifecycle of Locked, Released, Disputed, Resolved, and Cancelled.

## Finality Delay

A payment-settlement feature that holds merchant funds for a configurable period after payment completion before releasing them. Payments below a configurable `min_amount_threshold` bypass the delay and settle immediately.

## Horizon

The Stellar network's HTTP API used by off-chain services to subscribe to ledger events (such as `EscrowCreated`), enabling real-time notifications without polling.

## Multisig

Multi-signature governance model used for admin operations across contracts. Actions require a configurable number of approvals from an admin set before they execute, following a proposal-based workflow.

## Reason Code

A type-safe enum (`RefundReasonCode`) that categorises refunds into structured reasons: `ProductDefect`, `NonDelivery`, `DuplicateCharge`, `Unauthorized`, `CustomerRequest`, and `Other`.

## Spend Limits

Per-customer rolling spending caps enforced during payment creation. Each limit specifies a maximum amount within a configurable time window; the counter resets when the window elapses.

## Sub-Account

A labelled partition of an escrow that can be funded and released independently. A parent escrow cannot be fully released until all its sub-accounts have been released.

## Threshold

A configurable minimum value used in multiple contexts: the number of multisig approvals required to execute a proposal, the cumulative weight (in basis points) needed to release a multi-party escrow, or an inactivity period before reputation decay begins.

## WASM

WebAssembly — the compiled bytecode format (`.wasm`) to which Soroban smart contracts are compiled for deployment on the Stellar network, using the `wasm32-unknown-unknown` Rust target.
