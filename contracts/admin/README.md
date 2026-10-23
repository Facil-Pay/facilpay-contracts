# Admin Contract

Part of the [FacilPay smart contracts](../../README.md) suite on Stellar/Soroban.

## Purpose

The admin contract gates privileged operations across the other contracts. It acts as a central authority that can coordinate emergency actions — such as pausing and unpausing the payment, escrow, and refund contracts in a single Soroban call, or pausing one named function on one child contract — and can update the child contract addresses it points at.

## Role & Permission Model

The contract defines two distinct roles, both stored in instance storage and both set **once** during `initialize`:

### Admin Set (multisig)

- Privileged calls are controlled by an **admin set** with a **signature threshold** (see [Multisig Admin](#multisig-admin)).
- `initialize(admin, pauser, payment_contract, escrow_contract, refund_contract)` creates a one-member set `[admin]` with threshold 1.
- Calling `initialize` a second time returns `Error::AlreadyInitialized`.
- The admin address must authorize the `initialize` call via Soroban authentication (`require_auth()`).
- Only admin-set members may update child contract addresses, upgrade the coordinator, or change the admin set.
- `get_admin()` returns the first member of the set.

### Pauser Address

- The pauser is a single `Address` stored in contract instance storage under the `Pauser` data key.
- It is set during `initialize` alongside the admin and cannot be changed after initialization.
- The pauser stays a **single key** even when the admin set is a multisig, so emergency response stays fast.
- The pauser is the only role allowed to call `emergency_pause_all`, `emergency_unpause_all`, `emergency_pause_function` and `emergency_unpause_function`. A bounded pause can also be lifted by anyone through `expire_emergency_pause` once it has reached its expiry, and extended by an admin-set member through `extend_emergency_pause` (see [Bounded Emergency Pause](#bounded-emergency-pause)).
- The pauser address must also be an admin/multisig member of each child contract. Otherwise the child contract's `pause_contract` / `unpause_contract` / `pause_function` / `unpause_function` call rejects the pauser and the emergency call fails.

### Permission Checks

Every privileged function performs two authorization checks:

1. **Authentication** — the caller must pass Soroban's `require_auth()` for the supplied address.
2. **Authorization** — the caller's address must match the stored role address exactly (admin or pauser, depending on the function). A mismatch returns `Error::Unauthorized`.

### Privileged Operations

| Function | Description | Role Required |
|---|---|---|
| `initialize(admin, pauser, payment_contract, escrow_contract, refund_contract)` | Configures the contract with the admin, pauser, and child contract addresses. | Admin (sets both roles) |
| `emergency_pause_all(pauser, reason, max_pause_seconds)` | Pauses the payment, escrow, and refund contracts in one call, optionally with an expiry. See [Bounded Emergency Pause](#bounded-emergency-pause). | Pauser |
| `emergency_unpause_all(pauser)` | Unpauses the payment, escrow, and refund contracts in one call and clears any pause expiry. | Pauser |
| `extend_emergency_pause(admin, additional_seconds)` | Pushes back the expiry of a bounded pause before it is reached. | Admin member (any threshold) |
| `expire_emergency_pause()` | Unpauses all children once a bounded pause has reached its expiry. | Anyone (children still check the pauser, see below) |
| `emergency_pause_function(pauser, target, function_name, reason)` | Pauses one function on the targeted child contract only. | Pauser |
| `emergency_unpause_function(pauser, target, function_name)` | Unpauses one function on the targeted child contract only. | Pauser |
| `set_payment_contract(admin, payment_contract)` | Updates the stored payment contract address. | Admin member, threshold 1 only |
| `set_escrow_contract(admin, escrow_contract)` | Updates the stored escrow contract address. | Admin member, threshold 1 only |
| `set_refund_contract(admin, refund_contract)` | Updates the stored refund contract address. | Admin member, threshold 1 only |
| `upgrade(admin, new_wasm_hash)` | Replaces the coordinator WASM. | Admin member, threshold 1 only |
| `upgrade_children(admin, payment_hash, escrow_hash, refund_hash)` | Pauses, upgrades and unpauses all three child contracts in one call. See [Coordinated Child Upgrades](#coordinated-child-upgrades). | Admin member, threshold 1 only |
| `set_admins(admin, admins, threshold)` | Replaces the admin set and threshold. | Admin member, threshold 1 only |
| `propose_action(proposer, action)` | Proposes a privileged action. | Admin member |
| `approve_action(approver, proposal_id)` | Approves an open proposal. | Admin member |
| `execute_action(executor, proposal_id)` | Runs a proposal that has reached the threshold. | Admin member |

### Read-Only Operations

| Function | Description | Returns |
|---|---|---|
| `get_admins()` | The admin set. `NotInitialized` before `initialize`. | `Vec<Address>` |
| `get_threshold()` | The signature threshold. `NotInitialized` before `initialize`. | `u32` |
| `get_proposal(proposal_id)` | A proposal by id. `ProposalNotFound` if unknown. | `AdminProposal` |
| `get_emergency_pause_expiry()` | When the current bounded pause may be expired, or `None` if no bounded pause is in effect. | `Option<u64>` |
| `get_incident_log(offset, limit)` | A page of the emergency-action incident log, oldest first. See [Incident Log](#incident-log). | `Vec<IncidentEntry>` |
| `get_incident_count()` | Total number of incident log entries. | `u64` |
| `get_schema_version()` | The coordinator's own schema version (`ADMIN_SCHEMA_VERSION`). | `u32` |
| `get_platform_versions()` | The coordinator's version and each child's `get_schema_version`. See [Platform Versions](#platform-versions). | `PlatformVersions` |
| `get_platform_status()` | Returns the current pause state of all three child contracts. If a child contract is unreachable, it is reported as `Unknown` rather than causing a panic. | `PlatformStatus` containing the address and pause status of each child. |

### `PlatformStatus` Return Type

The `get_platform_status()` function returns a `PlatformStatus` struct containing:

```
pub struct PlatformStatus {
    pub payment: ChildContractStatus,
    pub escrow: ChildContractStatus,
    pub refund: ChildContractStatus,
}
```

Each field is a `ChildContractStatus` enum that can be:

- `Payment(Address, bool)` — payment contract address and globally_paused flag
- `Escrow(Address, bool)` — escrow contract address and globally_paused flag
- `Refund(Address, bool)` — refund contract address and globally_paused flag
- `Unknown(Address)` — contract address is known but unreachable (no panic)

### Error Codes

| Code | Constant | Description |
|---|---|---|
| 1 | `AlreadyInitialized` | `initialize` was called more than once. |
| 2 | `NotInitialized` | A privileged function was called before `initialize`. |
| 3 | `Unauthorized` | The caller's address does not match the stored admin or pauser. |
| 4 | `InvalidFunctionName` | `emergency_pause_function` / `emergency_unpause_function` was called with an empty `function_name`. |
| 5 | `InvalidAdminSet` | The admin set is empty, has duplicates, or has more than `MAX_ADMINS` (20) members. |
| 6 | `InvalidThreshold` | The threshold is 0 or greater than the number of admins. |
| 7 | `ProposalNotFound` | No proposal has the given id. |
| 8 | `ProposalExpired` | The proposal's `expires_at` has passed. |
| 9 | `ProposalAlreadyExecuted` | The proposal has already been executed. |
| 10 | `AlreadyApproved` | The approver has already approved this proposal. |
| 11 | `ThresholdNotMet` | `execute_action` was called with fewer valid approvals than the threshold. |
| 12 | `MultisigRequired` | A direct privileged call was made while the threshold is above 1; use a proposal instead. |
| 13 | `NoPauseExpiry` | `extend_emergency_pause` / `expire_emergency_pause` was called with no bounded pause in effect. |
| 14 | `PauseNotExpired` | `expire_emergency_pause` was called before the pause expiry. |
| 15 | `PauseExpired` | `extend_emergency_pause` was called at or after the pause expiry. |
| 16 | `InvalidPauseDuration` | `max_pause_seconds` is `Some(0)`, `additional_seconds` is 0, or the new expiry overflows `u64`. |

## Multisig Admin

Privileged coordinator calls (child contract setters, `upgrade`, and changing the admin set) are controlled by an admin set with a signature threshold.

### Direct calls vs. proposals

- While the threshold is **1**, any admin member may call `set_payment_contract`, `set_escrow_contract`, `set_refund_contract`, `upgrade`, `upgrade_children` and `set_admins` directly. This is how a freshly initialized coordinator bootstraps a multisig.
- Once the threshold is **above 1**, those direct calls fail with `MultisigRequired`, and the action must go through `propose_action` → `approve_action` → `execute_action`.

### `AdminAction`

| Variant | Effect when executed |
|---|---|
| `SetPaymentContract(Address)` | Updates the stored payment contract. |
| `SetEscrowContract(Address)` | Updates the stored escrow contract. |
| `SetRefundContract(Address)` | Updates the stored refund contract. |
| `Upgrade(BytesN<32>)` | Replaces the coordinator WASM (emits `AdminContractUpgraded` with `upgraded_by` = executor). |
| `SetAdmins(Vec<Address>, u32)` | Replaces the admin set and threshold (emits `AdminSetUpdated`). |
| `UpgradeChildren(BytesN<32>, BytesN<32>, BytesN<32>)` | Upgrades the payment, escrow and refund contracts (in that order of hashes) exactly like `upgrade_children`, with `upgraded_by` = executor. If any child call fails, the whole `execute_action` reverts and the proposal stays unexecuted. |

### `set_admins(admin, admins, threshold)`

| Parameter | Type | Description |
|---|---|---|
| `admin` | `Address` | Current admin member. Must authorize. |
| `admins` | `Vec<Address>` | New admin set: 1–20 unique addresses. |
| `threshold` | `u32` | Approvals needed to execute, `1..=admins.len()`. |

Errors: `NotInitialized`, `Unauthorized`, `MultisigRequired`, `InvalidAdminSet`, `InvalidThreshold`. Event: `AdminSetUpdated`.

### `propose_action(proposer, action) -> u64`

Creates a proposal and records the proposer's approval. The proposal expires `PROPOSAL_TTL_SECONDS` (7 days) after creation. `SetAdmins` proposals are validated up front.

Errors: `NotInitialized`, `Unauthorized`, `InvalidAdminSet`, `InvalidThreshold`. Event: `AdminProposalCreated`.

### `approve_action(approver, proposal_id)`

Adds an approval. Errors: `NotInitialized`, `Unauthorized`, `ProposalNotFound`, `ProposalAlreadyExecuted`, `ProposalExpired`, `AlreadyApproved`. Event: `AdminProposalApproved`.

### `execute_action(executor, proposal_id)`

Runs the action if approvals from **current** admin members reach the threshold. Approvals from addresses removed from the set since they approved do not count. Each proposal runs at most once.

Errors: `NotInitialized`, `Unauthorized`, `ProposalNotFound`, `ProposalAlreadyExecuted`, `ProposalExpired`, `ThresholdNotMet`. Event: `AdminProposalExecuted`, plus the action's own event where it has one.

### `AdminProposal`

```rust
pub struct AdminProposal {
    pub id: u64,
    pub action: AdminAction,
    pub proposer: Address,
    pub approvals: Vec<Address>,
    pub created_at: u64,
    pub expires_at: u64, // created_at + PROPOSAL_TTL_SECONDS; open while now <= expires_at
    pub executed: bool,
}
```

### Multisig events

| Event | Fields | Emitted by |
|---|---|---|
| `AdminProposalCreated` | `proposal_id: u64`, `proposer: Address`, `action: AdminAction`, `expires_at: u64` | `propose_action` |
| `AdminProposalApproved` | `proposal_id: u64`, `approver: Address`, `approvals: u32` | `approve_action` |
| `AdminProposalExecuted` | `proposal_id: u64`, `executed_by: Address` | `execute_action` |
| `AdminSetUpdated` | `admins: Vec<Address>`, `threshold: u32` | `set_admins`, executed `SetAdmins` proposals |

### Upgrading from a single-admin deployment

A coordinator initialized before multisig support only stored `Admin`. After upgrading, it is treated as a one-member set with threshold 1 until `set_admins` is called.

## Platform Versions

`get_platform_versions() -> PlatformVersions` returns the coordinator's schema version and each child's `get_schema_version()` in one call, so operators can check a deployment without querying each child.

```rust
pub struct PlatformVersions {
    pub admin: u32,           // ADMIN_SCHEMA_VERSION
    pub payment: Option<u32>,
    pub escrow: Option<u32>,
    pub refund: Option<u32>,
}
```

- No parameters, no auth.
- **Never fails:** a child whose address is not set (for example before `initialize`), or whose call fails, is reported as `None` instead of reverting.
- Errors: none. Events: none.

## Bounded Emergency Pause

A forgotten emergency pause could otherwise freeze the platform indefinitely. `emergency_pause_all` can be given a maximum duration, after which anyone may lift the pause.

### `emergency_pause_all(pauser, reason, max_pause_seconds)`

| Parameter | Type | Description |
|---|---|---|
| `pauser` | `Address` | Must authorize the call and match the stored pauser. |
| `reason` | `String` | Human-readable reason, passed to each child's `pause_contract`. |
| `max_pause_seconds` | `Option<u64>` | `Some(n)`: the pause may be expired once the ledger timestamp reaches `now + n`. `None`: no expiry; any expiry left by an earlier pause is cleared. |

Errors: `NotInitialized`, `Unauthorized`, `InvalidPauseDuration` (`Some(0)` or overflow). Calling it again while paused replaces the expiry.

### `extend_emergency_pause(admin, additional_seconds) -> u64`

Moves the expiry forward by `additional_seconds` and returns the new expiry. Allowed only while `now < expiry`. Any single admin-set member may extend, even when the threshold is above 1, because extending only keeps the platform paused, and the pauser can still lift the pause at any time with `emergency_unpause_all`.

| Parameter | Type | Description |
|---|---|---|
| `admin` | `Address` | Admin-set member. Must authorize. |
| `additional_seconds` | `u64` | Seconds to add to the current expiry. Must be non-zero. |

Errors: `NotInitialized`, `Unauthorized`, `NoPauseExpiry`, `PauseExpired`, `InvalidPauseDuration`. Event: `EmergencyPauseExtended`.

### `expire_emergency_pause()`

Once `now >= expiry`, calls `unpause_contract(pauser)` on the payment, escrow and refund contracts, clears the expiry and emits `EmergencyPauseAutoExpired`. The pause is no longer bounded afterwards, so a second call returns `NoPauseExpiry`.

The coordinator does not check who calls it. Each child contract still checks the **pauser's** authorization on its own `unpause_contract` call. The transaction must therefore carry the pauser's authorization entries for those three child calls. Those calls are not the root of the call tree, so the entries are non-root authorizations. The pauser can sign them in advance so that a keeper or any other account can submit the transaction. A signature can only be valid for as long as the network's maximum signature-expiration window allows.

Errors: `NoPauseExpiry`, `PauseNotExpired`, `NotInitialized`. Event: `EmergencyPauseAutoExpired`.

`emergency_unpause_all` also clears the expiry, so a pause the pauser lifts manually cannot be "expired" later.

### Events

| Event | Fields | Emitted by |
|---|---|---|
| `EmergencyPauseExtended` | `extended_by: Address`, `previous_expires_at: u64`, `new_expires_at: u64` | `extend_emergency_pause` |
| `EmergencyPauseAutoExpired` | `expires_at: u64`, `unpaused_at: u64` | `expire_emergency_pause` |

`EmergencyPauseAutoExpired` is distinct from the children's own unpause events, so indexers can tell an automatic expiry from a manual `emergency_unpause_all`.

## Function-Level Emergency Pause

Incident response sometimes needs to disable a single operation (for example refund processing) without halting the whole platform. The admin contract fans a function-level pause out to exactly one child contract.

### `ChildKind`

| Variant | Target |
|---|---|
| `Payment` | The stored payment contract |
| `Escrow` | The stored escrow contract |
| `Refund` | The stored refund contract |

### `emergency_pause_function(pauser, target, function_name, reason)`

Calls `pause_function(pauser, function_name, reason)` on the child selected by `target`. Other functions on that child, and all other child contracts, are left untouched; nothing is globally paused.

| Parameter | Type | Description |
|---|---|---|
| `pauser` | `Address` | Must authorize the call and match the stored pauser. |
| `target` | `ChildKind` | Which child contract to act on. |
| `function_name` | `String` | Name of the child function to pause, e.g. `"process_refund"`. Must be non-empty. |
| `reason` | `String` | Human-readable reason, recorded in the child's pause history. The escrow contract rejects an empty reason. |

Errors: `NotInitialized`, `Unauthorized`, `InvalidFunctionName`. If the child contract rejects the call (for example because the pauser is not one of its admins), the whole transaction fails.

### `emergency_unpause_function(pauser, target, function_name)`

Calls `unpause_function(pauser, function_name)` on the child selected by `target`.

| Parameter | Type | Description |
|---|---|---|
| `pauser` | `Address` | Must authorize the call and match the stored pauser. |
| `target` | `ChildKind` | Which child contract to act on. |
| `function_name` | `String` | Name of the child function to unpause. Must be non-empty. |

Errors: `NotInitialized`, `Unauthorized`, `InvalidFunctionName`.

### Events

In addition to the child contract's own `FunctionPausedEvent` / `FunctionUnpausedEvent`, the admin contract emits:

| Event | Fields | Emitted by |
|---|---|---|
| `EmergencyFunctionPausedEvent` | `target: ChildKind`, `function_name: String`, `paused_by: Address`, `reason: String`, `paused_at: u64` | `emergency_pause_function` |
| `EmergencyFunctionUnpausedEvent` | `target: ChildKind`, `function_name: String`, `unpaused_by: Address`, `unpaused_at: u64` | `emergency_unpause_function` |

## Incident Log

Post-incident reviews need a lasting record of every emergency action. The coordinator appends an `IncidentEntry` to a log in **persistent storage** each time one of these succeeds:

| Function | `action` | `target` | `function` | `reason` | `actor` |
|---|---|---|---|---|---|
| `emergency_pause_all` | `PauseAll` | `AllChildren` | `None` | the supplied reason | `Some(pauser)` |
| `emergency_unpause_all` | `UnpauseAll` | `AllChildren` | `None` | empty | `Some(pauser)` |
| `extend_emergency_pause` | `ExtendPause` | `AllChildren` | `None` | empty | `Some(admin)` |
| `expire_emergency_pause` | `PauseAutoExpired` | `AllChildren` | `None` | empty | `None` (anyone may call it) |
| `emergency_pause_function` | `PauseFunction` | `Child(target)` | `Some(function_name)` | the supplied reason | `Some(pauser)` |
| `emergency_unpause_function` | `UnpauseFunction` | `Child(target)` | `Some(function_name)` | empty | `Some(pauser)` |

Calls that fail (wrong caller, invalid arguments, a child rejecting the call) revert and leave no entry. Entries are never edited or removed.

### `IncidentEntry`

| Field | Type | Description |
|---|---|---|
| `id` | `u64` | Sequential ID, starting at 1. |
| `action` | `IncidentAction` | `PauseAll`, `UnpauseAll`, `ExtendPause`, `PauseAutoExpired`, `PauseFunction` or `UnpauseFunction`. |
| `target` | `IncidentTarget` | `AllChildren` for platform-wide actions, or `Child(ChildKind)` for function-level ones. |
| `function` | `Option<String>` | The child function, for function-level actions. |
| `reason` | `String` | The reason supplied with the action, or empty if the action takes none. |
| `actor` | `Option<Address>` | Who performed the action. `None` for a permissionless auto-expiry. |
| `at` | `u64` | Ledger timestamp of the action. |

### `get_incident_log(offset, limit) -> Vec<IncidentEntry>`

| Parameter | Type | Description |
|---|---|---|
| `offset` | `u64` | Number of entries to skip from the oldest. |
| `limit` | `u32` | Maximum number of entries to return. Capped at `MAX_INCIDENT_PAGE` (50). |

Returns entries in ID order. Returns an empty vector when `offset >= get_incident_count()` or `limit` is 0. Read-only, needs no authorization, and has no errors. To read the whole log, page with `offset += limit` until fewer than `limit` entries come back.

### `get_incident_count() -> u64`

The number of entries in the log. Read-only, no errors.

### Events

| Event | Fields | Emitted by |
|---|---|---|
| `IncidentLogged` | `incident_id: u64`, `action: IncidentAction`, `at: u64` | Every function that appends an entry, after its own event |

## Security Considerations

- Use an admin set with a threshold above 1 in production so no single key can rewire or upgrade the platform. The pauser is intentionally a single key; it should still be a multi-sig or governance contract address where possible.
- Because `emergency_pause_all` halts all child contracts, the pauser key should be treated as a high-value credential and stored securely (e.g., in a hardware wallet or threshold-signing scheme).
- The pauser must be an admin/multisig member of each child contract, otherwise the child's `pause_contract` call rejects it.
- The child contract addresses are **mutable**: the admin can point the contract at new payment, escrow, or refund contracts via `set_payment_contract`, `set_escrow_contract`, and `set_refund_contract`. Review these setters carefully, as they redirect all future emergency calls.
- The admin set can be changed through `set_admins` (threshold 1) or a `SetAdmins` proposal. The pauser role is permanent. Review deployment scripts carefully before calling `initialize`.

---

## See Also

- [Root README](../../README.md) — architecture overview and workspace setup
- [Payment Contract](../payment/README.md)
- [Escrow Contract](../escrow/README.md)
- [Refund Contract](../refund/README.md)

## Contract Upgrades

| Function | Parameters | Returns | Auth |
|---|---|---|---|
| `upgrade` | `admin: Address`, `new_wasm_hash: BytesN<32>` | `Result<(), Error>` | `admin` (admin-set member; threshold 1 only, otherwise propose `AdminAction::Upgrade`) |

Replaces the coordinator's WASM in place via `update_current_contract_wasm`. The stored admin, pauser and child contract addresses survive the upgrade. Upload the new WASM first; the new code runs from the next invocation.

**Errors:** `NotInitialized` (2) if the contract was never initialized, `Unauthorized` (3) if `admin` is not in the admin set, `MultisigRequired` (12) if the threshold is above 1.

**Event:** `AdminContractUpgraded { new_wasm_hash, upgraded_by }`.

## Coordinated Child Upgrades

Upgrading the payment, escrow and refund contracts in three separate transactions leaves the platform half-upgraded in between. `upgrade_children` does all three in one call.

| Function | Parameters | Returns | Auth |
|---|---|---|---|
| `upgrade_children` | `admin: Address`, `payment_hash: BytesN<32>`, `escrow_hash: BytesN<32>`, `refund_hash: BytesN<32>` | `Result<(), Error>` | `admin` (admin-set member; threshold 1 only, otherwise propose `AdminAction::UpgradeChildren`) |

Each hash is the hash of a WASM already uploaded to the network.

What the call does, in order:

1. Pauses each child (`pause_contract`) with reason `"coordinated upgrade"`.
2. Calls `upgrade` on the payment, escrow and refund contracts, emitting one `ChildContractUpgraded` per child.
3. Unpauses each child (`unpause_contract`); the new code handles this call.

A child that was already globally paused before the call (for example, during an emergency pause) is neither paused again nor unpaused, so an emergency pause stays in place.

**Any failure reverts the whole call.** If any child rejects a call (for example, a hash that was never uploaded, escrow's `MigrationInProgress`, or the coordinator not being authorized), the transaction fails with that child's error. No child is upgraded and none is left paused.

**Setup: the coordinator must be authorized on each child.** The coordinator calls the children as itself (`env.current_contract_address()`), so before using `upgrade_children`:

- **Payment and escrow:** add the coordinator's contract address to the child's admin multisig. Their `upgrade` also requires `required_signatures == 1`.
- **Refund:** make the coordinator's contract address the refund contract's admin.

**Errors:** `NotInitialized` (2) if the contract was never initialized, `Unauthorized` (3) if `admin` is not in the admin set, `MultisigRequired` (12) if the threshold is above 1. Child errors abort the call as described above.

**Events:**

| Event | Fields | Emitted by |
|---|---|---|
| `ChildContractUpgraded` | `child_kind: ChildKind`, `contract: Address`, `new_wasm_hash: BytesN<32>`, `upgraded_by: Address`, `upgraded_at: u64` | `upgrade_children` and executed `UpgradeChildren` proposals, once per child |

The children also emit their own pause, `ContractUpgraded` and unpause events.
