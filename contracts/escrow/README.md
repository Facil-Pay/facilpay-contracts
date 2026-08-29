# Escrow Contract

This contract manages secure, conditional fund holding for the Facil-Pay ecosystem, ensuring assets are only released when agreed-upon conditions are met by all parties.

## Public Functions

- create_escrow: Initializes a new escrow agreement with locked funds, terms, and designated participants.
- release_escrow: Releases the held funds to the recipient once the agreed-upon conditions are successfully met.
- dispute_escrow: Flags the escrow transaction for administrative arbitration if participants cannot reach a consensus.
- clawback: Reverts the funds back to the original sender if the escrow conditions expire or fundamentally fail.
- approve_multisig: Records an approval signature from a required participant for multi-signature escrow setups.
- add_observer: Assigns a read-only role to a specific address for auditing and compliance tracking.
- is_escrow_released: Verifies whether an escrow exists and has been released.
- is_escrow_disputed: Verifies whether an escrow exists and is currently in dispute.
- get_escrow_status: Queries the current lifecycle status of an escrow.
- get_escrow_parties: Queries the customer and merchant addresses for an escrow.
- get_escrow_amount: Queries the locked token amount held in an escrow.
- verify_escrow_participant: Verifies if a given address is a valid participant (customer or merchant) of an escrow.
- verify_observer_access: Verifies whether an observer has active, non-expired read-only access.

The escrow dispute flow has two separate timeout paths, and they apply in different dispute rounds.

- Escalation timeout applies while the escrow is in the Disputed state after a party escalates the dispute. `escalate_dispute` increments `escalation_level`, captures `escalated_at`, and adds a deadline at `now + escalation_timeout`. When that deadline is processed, `trigger_timeout_resolution` resolves the dispute under the configured `auto_resolve_in_favor_of` policy. This timeout is tied to the escalation event, not to a filed appeal.
- Appeal expiry applies only after the dispute has entered the Appeal round. An appeal can be filed only while the dispute round is not Final and the time since `dispute_started_at` is still within the 72-hour appeal window. The appeal stores `appeal_deadline = filed_at + 259200`, and if that deadline passes without a resolution, `expire_appeal` rejects the pending appeal, advances the dispute round to Final, and leaves the prior outcome as the effective final disposition.

These are distinct timers rather than one combined timeout. Escalation timeout is measured from the escalation timestamp on a disputed escrow, while appeal expiry is measured from the appeal filing deadline in the Appeal round. In practice, they are not both expected to fire for the same dispute state: the escalation path resolves the Disputed state before a valid appeal round is entered, and the appeal-expiry path only exists once an appeal has already been filed.
---

## Verification

The escrow contract provides a dedicated State Verification Interface and access-control verification mechanisms. These allow external contracts, off-chain integrators, and internal methods to verify escrow status, validate participant identities, verify observer access windows, and validate cryptographic evidence proofs.

### Verifier Roles

1. **Public & External Verifiers (Read-Only)**:
   - External contracts, cross-contract callers, client dApps, and indexers can verify escrow states and participant roles without requiring authentication or gas-intensive authorizations.
   - Used to verify state preconditions (e.g., confirming funds are `Released` before fulfilling an off-chain order, or ensuring an address is a registered participant before initiating a multi-party flow).

2. **Internal Contract Verifier (Guards & Access Control)**:
   - The contract internally executes verification checks before performing state modifications or exposing sensitive data:
     - **Escrow Inspection Guard (`get_escrow_details`)**: Verifies that the caller is either the `customer`, the `merchant`, or an active observer with valid access (`verify_observer_access`).
     - **Observer Management Guard (`add_observer` / `remove_observer`)**: Verifies that the granter is either an escrow participant (`verify_escrow_participant`) or a multisig admin.
     - **Dispute Evidence Verifier (`submit_evidence_with_proof`)**: Verifies cryptographic Keccak-256 Merkle proofs against pre-committed roots.
     - **Release Protection Guard (`release_escrow`)**: Verifies that the caller is an authorized admin or trusted bridge and rejects callers holding observer roles.

### Verification Functions and Inputs

| Function | Inputs | Return Type | Description |
| :--- | :--- | :--- | :--- |
| `is_escrow_released` | `escrow_id: u64` | `bool` | Returns `true` if the escrow exists and its status is `Released`. Returns `false` for non-existent IDs or unreleased escrows. |
| `is_escrow_disputed` | `escrow_id: u64` | `bool` | Returns `true` if the escrow exists and its status is `Disputed`. Returns `false` for non-existent IDs or non-disputed escrows. |
| `get_escrow_status` | `escrow_id: u64` | `Result<EscrowStatus, Error>` | Returns the exact `EscrowStatus` (`Locked`, `Released`, `Disputed`, `Resolved`, etc.) or `EscrowNotFound` if not found. |
| `get_escrow_parties` | `escrow_id: u64` | `Result<(Address, Address), Error>` | Returns the `(customer, merchant)` tuple or `EscrowNotFound`. |
| `get_escrow_amount` | `escrow_id: u64` | `Result<i128, Error>` | Returns the locked token amount or `EscrowNotFound`. |
| `verify_escrow_participant` | `escrow_id: u64`, `address: Address` | `bool` | Returns `true` if `address` matches either `escrow.customer` or `escrow.merchant`. Returns `false` for non-existent IDs or non-matching addresses. |
| `verify_observer_access` | `escrow_id: u64`, `observer: Address` | `bool` | Returns `true` if `observer` has an active grant where `now < expires_at`. Returns `false` if unassigned or expired. |
| `get_escrow_details` | `caller: Address`, `escrow_id: u64` | `Result<Escrow, Error>` | Requires `caller.require_auth()`. Returns full `Escrow` struct if caller passes participant or observer verification; returns `Unauthorized` otherwise. |
| `submit_evidence_with_proof` | `caller: Address`, `escrow_id: u64`, `evidence: Bytes`, `proof: Vec<BytesN<32>>`, `leaf_index: u32` | `Result<(), Error>` | Verifies `keccak256(evidence)` against committed Merkle root using proof and leaf index. Returns `InvalidMerkleProof` if verification fails. |

### State Effects: Passed vs. Failed Verification

- **Read-Only Verification (`is_escrow_released`, `is_escrow_disputed`, `verify_escrow_participant`, `verify_observer_access`)**:
  - **Passed (`true`)**: Indicates the condition is satisfied. No contract state or storage is altered.
  - **Failed (`false`)**: Indicates the condition is not satisfied (e.g., escrow does not exist, status does not match, or address is not a participant). No state is modified and no error is thrown.
- **State Query Verification (`get_escrow_status`, `get_escrow_parties`, `get_escrow_amount`)**:
  - **Passed (`Ok(value)`)**: Returns requested escrow metadata. No state mutations occur.
  - **Failed (`Err(EscrowError::NotFound)`)**: Operation returns a not-found error without modifying contract state.
- **Access Control Verification (`get_escrow_details`, `add_observer`, `remove_observer`)**:
  - **Passed**: The caller is authorized to view sensitive escrow records or grant/revoke observer roles.
  - **Failed**: Fails with `Error::Basic(BasicError::Unauthorized)` or `Error::Basic(BasicError::NotAnAdmin)`. Transaction reverts with zero state mutations.
- **Observer Expiry Verification**:
  - `verify_observer_access` automatically begins returning `false` once `env.ledger().timestamp() >= observer.expires_at`.
  - Expired observer entries remain stored in historical observer records (`get_observers`) but lose read access to `get_escrow_details` immediately without requiring an on-chain cleanup transaction.
- **Merkle Proof Evidence Verification**:
  - **Passed**: Evidence entry is validated against `EvidenceCommitment.merkle_root` and recorded on-chain in dispute history.
  - **Failed**: Reverts with `Error::Basic(BasicError::InvalidMerkleProof)`. No evidence is stored and no event is emitted.

---

## Admin Succession

Succession lets the current multisig admin set hand control of the contract to a new admin address after a time delay, without requiring the successor to already be part of the multisig.

### Designating a Successor

Any existing admin can designate a successor:

```
designate_successor(admin, successor, delay_seconds) -> ()
```

- `admin` must be a current member of the multisig admin set (enforced via `require_auth` and an admin-list check)
- `successor` cannot be the zero address (`InvalidAddress`) and cannot be the same address as `admin` (`SameBeneficiary`)
- Only one pending (non-activated) succession plan may exist at a time — designating while a plan is already pending returns `SuccessionPlanExists`
- The plan becomes activatable at `activatable_after = now + delay_seconds`

### Activating Succession

Once the delay has elapsed, the designated successor — not the original admin — activates the plan themselves:

```
activate_succession(successor) -> ()
```

- Must be called and authorized by the `successor` address named in the plan; any other caller gets `Unauthorized`
- Fails with `NotReady` if called before `activatable_after`
- Fails with `AlreadyProcessed` if the plan was already activated
- On success, the successor is added to the multisig admin set (if not already present) and the plan is marked `activated`

### Revoking a Pending Succession

Any current admin (not only the one who designated it) can revoke a plan before it activates:

```
revoke_succession(admin) -> ()
```

- `admin` must be a current multisig admin
- Fails with `AlreadyProcessed` if the plan has already been activated — an activated succession cannot be undone by revocation
- Removes the stored plan entirely, allowing a new one to be designated

### Interaction with Disputes

Succession only adds a new address to the multisig admin set — it does not read, lock, or modify any escrow, dispute, or appeal state. Designating, activating, or revoking a succession plan has no effect on disputes that are in flight, and an in-flight dispute has no effect on succession: the two are independent. The newly added admin can act on future admin-gated calls (e.g. `resolve_dispute`, `set_batch_limit`) once activated, exactly like any other admin.

### Queries

| Function                  | Returns                              |
| -------------------------- | ------------------------------------ |
| `get_succession_plan()`   | `Option<SuccessionPlan>` — the current pending or last-activated plan, if any |

---

## Sub-Accounts

Sub-accounts allow a merchant to split a single escrow into smaller, independently releasable allocations. Each sub-account represents a designated portion of the parent escrow's funds that can be released to the merchant on its own schedule, without requiring the entire escrow to be released at once.

### What a Sub-Account Represents

A sub-account is a child record of an existing escrow. It holds:

- An **amount** — the portion of the parent escrow's funds allocated to this sub-account
- A **label hash** — a 32-byte identifier for off-chain categorisation (e.g. milestone ID, deliverable reference)
- A **released** flag — whether funds have been transferred to the merchant
- An optional **fee override** — a per-sub-account fee in basis points that overrides the parent escrow's fee

Sub-accounts do **not** store a customer or merchant address directly. The merchant is inherited from the parent escrow at call time. The customer has no role in sub-account operations; all customer interaction happens at the parent escrow level.

### Creating a Sub-Account

Only the **merchant** of the parent escrow can create sub-accounts:

```
create_sub_account(merchant, escrow_id, label_hash, amount, fee_bps_override) -> sub_id
```

- `merchant` must be the same address stored on the parent escrow (enforced via `require_auth` and address check)
- The combined allocation of all sub-accounts (including the new one) must not exceed the parent escrow's locked amount — creating a sub-account that would over-allocate returns `SubAccountFundingExceedsEscrow`
- `fee_bps_override` is optional: `None` inherits the parent escrow's fee; `Some(0)` means zero fees on this sub-account; `Some(1000)` means a 10% fee
- Sub-accounts are assigned sequential IDs per escrow (starting at 1) and **cannot be deleted** once created

### Funding a Sub-Account

After creation, a sub-account's allocation can be increased:

```
fund_sub_account(funder, escrow_id, sub_id, amount)
```

- Any address can fund a sub-account (the `funder` must authorize the call)
- The total allocation across all sub-accounts is re-validated against the parent escrow amount on every funding call
- Returns `SubAccountFundingExceedsEscrow` if the increase would exceed the parent escrow

### Releasing a Sub-Account

Only the **admin** can release a sub-account:

```
release_sub_account(admin, escrow_id, sub_id)
```

- Transfers funds from the sub-account to the merchant, minus any applicable fee
- The effective fee is resolved as: `sub.fee_bps_override.unwrap_or(parent_escrow.fee_bps)`
- The fee portion is sent to the configured `fee_recipient`; the remainder goes to the merchant
- After release, `sub.released` is set to `true` — a released sub-account **cannot be released again** (`SubAccountAlreadyReleased`)

### Parent Escrow Release Guard

The parent escrow **cannot be released** while any sub-account remains unreleased. The `release_escrow` function checks all sub-accounts and returns `InvalidStatus` if any sub-account has `released == false`.

This enforces the invariant that all sub-accounts must be individually resolved before the parent escrow can be fully released. The typical workflow is:

1. Create escrow with locked funds
2. Create sub-accounts for each milestone/deliverable
3. Admin releases each sub-account as milestones are completed
4. Once all sub-accounts are released, the parent escrow can be released (if any remainder exists)

### Fee Override

Each sub-account can override the parent escrow's fee independently:

```
set_sub_account_fee_override(merchant, escrow_id, sub_id, fee_bps_override)
```

- Only the escrow's merchant can call this
- The sub-account must not already be released
- `fee_bps_override` can be `None` (inherit parent fee) or `Some(value)` (use `value` as the fee in basis points)

### Queries

| Function                                          | Returns                                        |
| ------------------------------------------------- | ---------------------------------------------- |
| `get_sub_account(escrow_id, sub_id)`              | `Option<EscrowSubAccount>` — a single record   |
| `list_sub_accounts(escrow_id)`                    | `Vec<EscrowSubAccount>` — all sub-accounts     |

### Error Codes

| Error                                | Code | Meaning                                                              |
| ------------------------------------ | ---- | -------------------------------------------------------------------- |
| `SubAccountNotFound`                | 214  | No sub-account exists for the given escrow/sub ID pair               |
| `SubAccountAlreadyReleased`         | 215  | Attempted to release or modify a sub-account that was already released |
| `SubAccountFundingExceedsEscrow`    | 216  | Total sub-account allocations would exceed the parent escrow amount  |

---

## Swap Flow

Escrow swaps are a controlled token conversion for a locked escrow. They are designed for a merchant or admin to convert an escrow's current `source_token` into a new `target_token` while preserving the escrow's lifecycle and preventing accidental low-value swaps.

### Who can configure and execute a swap

- `configure_escrow_swap(merchant_or_admin, escrow_id, target_token, min_output, oracle)`
  - allowed for the escrow merchant or any current admin
  - the escrow must still be in the `Locked` state
  - the swap config is stored per escrow and can only be executed once
- `execute_escrow_swap(caller, escrow_id)`
  - allowed for the same callers as above
  - checks the previously configured oracle and minimum output threshold
  - updates the escrow token and amount in place after successful execution

### Supported tokens and pricing

- The escrow's source token is the token already stored on the escrow record.
- The target token is arbitrary as long as it is a valid `Address`; the contract does not hard-code a whitelist. In practice, this is used for a configured conversion path such as USDC -> EURC or USDC -> XLM depending on the oracle and target address.
- Pricing is taken from the configured oracle via `get_rate()`, which returns a `1e7` fixed-point value. The contract computes:

```rust
output_amount = (escrow.amount * rate) / 10_000_000;
```

- `min_output_amount` acts as slippage protection. If `output_amount < min_output_amount`, execution reverts with `SwapOutputBelowMinimum` and the escrow is not mutated.
- The swap is not a true on-chain AMM swap; it is a contract-level conversion that updates the escrow's recorded `token` and `amount` to the newly computed value once the oracle output is accepted.

### Execution guarantees

- `SwapAlreadyExecuted` rejects a second execution of the same escrow swap.
- `SwapConfigNotFound` rejects attempts to execute before configuration.
- `Unauthorized` rejects non-merchant, non-admin callers.
- The escrow's status must remain `Locked` during execution; once executed, the change is persisted to storage and the config is marked as executed.

---

## Health Monitoring

The escrow contract exposes a health check for off-chain monitoring and admin review rather than for on-chain settlement. The public surface is `get_escrow_health(escrow_id)` and `get_stale_escrows(limit)`, backed by `set_stale_threshold(...)`.

### What it verifies

`get_escrow_health` classifies a single escrow using the configured thresholds:

- `Disputed` if the escrow status is already in dispute
- `Expired` if `expiry_timestamp != 0` and `now >= expiry_timestamp`
- `NearExpiry` if the escrow is still active but within the configured safety window
- `Stale` if the escrow has been inactive longer than `inactivity_seconds`
- `Healthy` otherwise

The health report also includes:

- `last_activity` — the escrow's last recorded activity timestamp
- `seconds_until_expiry` — the remaining time until expiry, or `None` if the escrow has no expiry

### Intended caller

This is primarily for:

- off-chain monitoring services and indexers
- admin dashboards checking stale or near-expiry escrows
- operational automation that decides whether to escalate or investigate a lock

The contract is intentionally read-only here; health checks do not mutate escrow state or resolve disputes. They are a verification and reporting utility, not an approval or release path.

### Threshold configuration

`set_stale_threshold(admin, config)` stores:

- `inactivity_seconds` — how long an escrow can remain idle before it is considered stale
- `near_expiry_buffer_seconds` — how close to expiry it must be before it is labelled `NearExpiry`

If no threshold has been configured, `get_escrow_health` and `get_stale_escrows` panic with `StaleThresholdNotConfigured`.

---
[⬅ Back to Main README](../../README.md)

