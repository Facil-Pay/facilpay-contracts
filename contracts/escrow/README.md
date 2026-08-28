# Escrow Contract

This contract manages secure, conditional fund holding for the Facil-Pay ecosystem, ensuring assets are only released when agreed-upon conditions are met by all parties.

## Public Functions

- create_escrow: Initializes a new escrow agreement with locked funds, terms, and designated participants.
- release_escrow: Releases the held funds to the recipient once the agreed-upon conditions are successfully met.
- dispute_escrow: Flags the escrow transaction for administrative arbitration if participants cannot reach a consensus.
- clawback: Reverts the funds back to the original sender if the escrow conditions expire or fundamentally fail.
- approve_multisig: Records an approval signature from a required participant for multi-signature escrow setups.
- add_observer: Assigns a read-only role to a specific address for auditing and compliance tracking.

The escrow dispute flow has two separate timeout paths, and they apply in different dispute rounds.

- Escalation timeout applies while the escrow is in the Disputed state after a party escalates the dispute. `escalate_dispute` increments `escalation_level`, captures `escalated_at`, and adds a deadline at `now + escalation_timeout`. When that deadline is processed, `trigger_timeout_resolution` resolves the dispute under the configured `auto_resolve_in_favor_of` policy. This timeout is tied to the escalation event, not to a filed appeal.
- Appeal expiry applies only after the dispute has entered the Appeal round. An appeal can be filed only while the dispute round is not Final and the time since `dispute_started_at` is still within the 72-hour appeal window. The appeal stores `appeal_deadline = filed_at + 259200`, and if that deadline passes without a resolution, `expire_appeal` rejects the pending appeal, advances the dispute round to Final, and leaves the prior outcome as the effective final disposition.

These are distinct timers rather than one combined timeout. Escalation timeout is measured from the escalation timestamp on a disputed escrow, while appeal expiry is measured from the appeal filing deadline in the Appeal round. In practice, they are not both expected to fire for the same dispute state: the escalation path resolves the Disputed state before a valid appeal round is entered, and the appeal-expiry path only exists once an appeal has already been filed.
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
[⬅ Back to Main README](../../README.md)
