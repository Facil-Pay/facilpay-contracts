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

## Dispute Collateral

Collateral is a security deposit required from the party initiating a dispute. It ensures that disputing parties have skin in the game and discourages frivolous disputes.

**Configuration**

- **`set_dispute_config(admin, config)`**: Sets the dispute collateral parameters. The `DisputeConfig` includes:
  - `collateral_token`: The token contract address used for collateral deposits
  - `collateral_amount`: The exact amount of collateral required to initiate a dispute
  - `collateral_enabled`: Whether collateral is required (`true`) or disabled (`false`)

**How Collateral Works**

When a party calls `dispute_escrow`:
- If `collateral_enabled` is `true`, the disputing party must have `collateral_amount` of `collateral_token` in their balance
- The collateral amount is automatically transferred from the disputing party to the escrow contract
- The collateral is held alongside the escrowed funds until the dispute is resolved
- **`get_dispute_collateral(escrow_id)`**: Returns the collateral record for a disputed escrow, including `amount` and `disputing_party`

**Collateral Return vs. Forfeiture**

The fate of the collateral depends on the dispute resolution:

- **Returned to disputing party**: If the dispute is resolved in favor of the disputing party, the collateral is returned to them along with their portion of the escrowed funds. For example, if a customer disputes and wins, they receive both the escrow amount and their collateral back.
- **Forfeited to counterparty**: If the dispute is resolved against the disputing party, the collateral is forfeited to the other party. For example, if a merchant disputes and loses, their collateral is transferred to the customer.

**Disabling Collateral**

- If `collateral_enabled` is set to `false`, disputes can be initiated without any collateral deposit
- When disabled, `get_dispute_collateral` returns an error for escrows with no collateral
- This allows for optional collateral enforcement based on the use case

**Interaction with Dispute Evidence Deadline**

- Collateral is deposited at the time `dispute_escrow` is called, which is independent of the evidence submission deadline
- The evidence deadline governs when evidence can be submitted, but does not affect the collateral requirement
- Collateral is held until dispute resolution regardless of evidence submission timing

---

## Beneficiary Transfer

Beneficiary transfer allows the merchant (beneficiary) of an escrow to be reassigned to a different address. This is useful for scenarios such as business acquisitions, merchant account changes, or internal routing adjustments.

**Authorization**

- **`transfer_escrow_beneficiary(caller, escrow_id, new_beneficiary)`**: Transfers the beneficiary (merchant) role to a new address.
- Only the **current merchant** or the **admin** is authorized to initiate a transfer. Any other caller receives `Unauthorized`.
- The original beneficiary does **not** need to consent to the transfer — authorization is determined solely by the caller being the current merchant or admin.

**Transfer Restrictions**

- **Same beneficiary rejected**: Transferring to the same address as the current beneficiary returns `SameBeneficiary`
- **Disputed escrow blocked**: If the escrow is in `Disputed` status, beneficiary transfer is blocked with `TransferNotAllowed`
- **Resolved escrow blocked**: If the escrow has already been resolved (after a dispute), beneficiary transfer is blocked with `TransferNotAllowed`
- Transfers are only allowed on escrows in active states (e.g., `Locked`)

**Transfer History**

- Every successful beneficiary transfer is recorded in an append-only history log
- **`get_transfer_history(escrow_id)`**: Returns the complete transfer history for an escrow, including:
  - `from`: The previous beneficiary address
  - `to`: The new beneficiary address
  - Timestamp of the transfer
- The history is empty before any transfers occur
- Multiple transfers can be chained (e.g., A → B → C), with each step recorded separately

**Interaction with Disputes**

- Once an escrow enters `Disputed` status, no further beneficiary transfers are allowed
- If a dispute is resolved, the escrow remains blocked for transfers even after resolution
- This ensures that the beneficiary at the time of dispute initiation remains the party involved in dispute resolution
- Transfers should be completed before any dispute is raised if a beneficiary change is needed

---

## Timelock vs. Hold Period

The escrow contract includes two time-based mechanisms: **timelock** and **hold period**. These serve different purposes and operate independently.

### Timelock

Timelock is a governance mechanism that delays the execution of sensitive administrative actions. It applies to queued actions such as dispute resolution and force release.

**Configuration**

- **`set_timelock_config(admin, config)`**: Sets the timelock parameters. The `TimeLockConfig` includes:
  - `delay`: Minimum delay in seconds before a queued action can be executed (must be between 1 hour and 7 days)
  - `grace_period`: Additional grace period in seconds after the delay expires

**How Timelock Works**

- **`queue_action(admin, escrow_id, action_type, data)`**: Queues an action for delayed execution. Returns an `action_id`.
- The action cannot be executed until the `delay` period has elapsed since queuing
- Attempting to execute before the delay returns `NotReady`
- **`cancel_queued_action(admin, action_id)`**: Cancels a queued action before it executes
- **`get_queued_action(action_id)`**: Retrieves the status of a queued action

Timelock is designed for admin-gated actions that require a cooling-off period, such as resolving disputes in favor of one party or force-releasing funds.

### Hold Period

Hold period is a customer protection mechanism that prevents premature refunds. It applies to the customer's ability to refund an escrow.

**How Hold Period Works**

- When creating an escrow with `create_escrow`, a `hold_period_seconds` parameter can be specified
- The customer cannot call `refund_escrow` until the hold period has elapsed since escrow creation
- Attempting to refund before the hold period expires fails
- Once the hold period elapses, the customer can refund the escrow unconditionally (if not disputed)

Hold period is designed to give merchants a reasonable window to fulfill obligations before customers can unilaterally reclaim funds.

### Interaction Between Timelock and Hold Period

**Independent Mechanisms**

- Timelock and hold period are **independent time windows** that do not stack or interfere with each other
- Timelock applies to **admin actions** (queued actions like dispute resolution)
- Hold period applies to **customer actions** (refund requests)
- They operate on different callers and different operations

**No Precedence Conflict**

- There is no scenario where both mechanisms apply to the same operation on the same escrow
- A customer refund is governed only by hold period (and dispute status)
- An admin action (e.g., dispute resolution) is governed only by timelock
- The two do not compete or take precedence — they simply apply to different operations

**Example Timeline**

1. Escrow created at t=0 with hold_period=500s
2. At t=100, customer cannot refund (hold period not elapsed)
3. At t=200, admin queues a dispute resolution action with timelock delay=7200s
4. At t=600, customer can now refund (hold period elapsed) — timelock does not block customer refunds
5. At t=7400, admin can execute the queued action (timelock delay elapsed) — hold period does not block admin actions

**Summary**

- **Timelock**: Delays admin actions for governance purposes
- **Hold period**: Delays customer refunds for protection purposes
- **Independence**: They apply to different operations and do not interact
- **No stacking**: They are separate windows, not cumulative constraints

---
[⬅ Back to Main README](../../README.md)

