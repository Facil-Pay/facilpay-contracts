# Escrow Storage Migration Runbook

`contracts/escrow` ships a **three-step, admin-driven migration** that upgrades stored escrow
records from schema version `1` to schema version `2`. It is deliberately different from the
single-call `migrate_schema` used by `contracts/payment` and `contracts/refund`
(see [Storage Schema Versioning](./STORAGE_VERSIONING.md)): the escrow contract migrates records
**in batches**, so a large escrow table can be upgraded across several transactions instead of one
oversized one.

Every name, parameter, error code and behaviour below is taken from the current source,
[`contracts/escrow/src/lib.rs`](../contracts/escrow/src/lib.rs).

---

## At a glance

| Step | Function | Caller | Net effect on storage |
| --- | --- | --- | --- |
| 1. Begin | `begin_migration(admin)` | Escrow admin | Writes `EscrowMigrationStatus { in_progress: true, migrated_count: 0, total_count: <escrow counter>, started_at: <now>, completed_at: None }` |
| 2. Migrate | `migrate_escrow(admin, escrow_id)` or `migrate_escrow_batch(admin, escrow_ids)` | Escrow admin | Re-writes each escrow record in the current format, marks `EscrowMigrated(escrow_id) = true`, advances `migrated_count` |
| 3. Complete | `complete_migration(admin)` | Escrow admin | Sets `in_progress = false`, `completed_at = <now>`, and writes `ConfigKey::SchemaVersion = 2` |

Read-only observation at any point: `get_migration_status()`.

---

## Who may call it

All four state-changing entry points enforce the same two gates before doing any work:

1. **Authorization** — `admin.require_auth()` runs first, then the address must be a member of the
   configured multisig admin set (`get_multisig_config().admins`). A non-admin address fails with
   `Error::Basic(BasicError::NotAnAdmin)` (**101**).
2. **Pause gate** — `require_not_paused(env, "<function_name>")` runs next. If the contract is
   globally paused, or the specific function name is on the paused-function list, the call fails
   with `Error::Basic(BasicError::ContractPaused)` (**103**). This means a global pause also halts a
   migration in progress — unpause before continuing.

`get_migration_status()` requires no authorization and never panics.

---

## Step 1 — `begin_migration(admin)`

```rust
pub fn begin_migration(env: Env, admin: Address) -> Result<(), Error>
```

Opens a migration window and snapshots how many escrows must be migrated. It:

1. Authorizes `admin` and checks the pause gate (see above).
2. Rejects the call with `Error::Basic(BasicError::SchemaAlreadyAtTarget)` (**114**) when
   `get_schema_version() >= MIGRATION_TARGET_SCHEMA_VERSION` (target is **2**, initial is **1**).
3. Rejects the call with `Error::Basic(BasicError::MigrationNotStarted)` (**106**) when a previous
   migration has already **completed** (a stored status with `in_progress == false` and
   `completed_at.is_some()`).
4. Reads `total_count` from the escrow counter (`EscrowKey::Counter`) and writes a fresh
   `MigrationStatus`:

```rust
MigrationStatus {
    in_progress: true,
    migrated_count: 0,
    total_count,
    started_at: env.ledger().timestamp(),
    completed_at: None,
}
```

**Notes**

- Calling `begin_migration` again **while a migration is in progress** does not error; it resets
  `migrated_count` to `0` and re-snapshots `total_count` with a new `started_at`. Every escrow that
  was already marked `EscrowMigrated` stays marked, so those records are skipped (single mode) or
  silently skipped (batch mode) on the re-run. Prefer one clean `begin` → `complete` cycle.
- If `total_count` is `0`, the migration can be completed immediately without calling either
  migrate function.

---

## Step 2 — migrate the records

Two equivalent entry points; use the batch form for anything but a single straggler.

### `migrate_escrow(admin, escrow_id)`

```rust
pub fn migrate_escrow(env: Env, admin: Address, escrow_id: u64) -> Result<(), Error>
```

- Authorizes/pause-checks `admin`, then requires an **active** migration: a missing status or
  `in_progress == false` both fail with `Error::Basic(BasicError::MigrationNotStarted)` (**106**).
- Fails with `Error::Basic(BasicError::AlreadyMigrated)` (**107**) if
  `EscrowMigrated(escrow_id)` is already `true`.
- Fails with `Error::Escrow(EscrowError::NotFound)` (**200**) if the escrow record does not exist.
- Otherwise reads the `Escrow` record, writes it back in the current format, sets
  `EscrowMigrated(escrow_id) = true`, and increments `migrated_count` by one.

### `migrate_escrow_batch(admin, escrow_ids)`

```rust
pub fn migrate_escrow_batch(env: Env, admin: Address, escrow_ids: Vec<u64>) -> Result<u32, Error>
```

- Applies the same authorization, pause gate and active-migration checks as `migrate_escrow`.
- For each id **in order**:
  - already-migrated ids are **skipped silently** (no `AlreadyMigrated` error),
  - ids with no stored escrow are **skipped silently** (no `NotFound` error),
  - everything else is re-written and marked migrated.
- Returns the **number of records actually migrated** in this call and adds exactly that number to
  `migrated_count`. It is *not* the length of `escrow_ids`, so always use the return value to track
  progress.

### Choosing a batch size

The escrow contract does **not** enforce a hard cap on `escrow_ids` — unlike `batch_release_escrows`,
which rejects more than 20 ids with `BatchReleaseSizeLimitExceeded`. The practical limit is the
Soroban transaction's resource budget: every id in the batch reads (and, when un-migrated, writes)
one escrow entry, and the whole invocation is rejected if it exceeds the ledger limits.

A safe operating procedure:

1. Start with a **small batch (10–20 ids)** — the same order of magnitude as the contract's own
   release cap of 20 — and confirm it lands.
2. Increase the size (40, 80, …) until an invocation fails or approaches the ledger's CPU/read-write
   limits, then step back one notch and use that as the standard batch size.
3. Re-send ids freely: repeated ids and unknown ids are skipped, and the returned count tells you how
   much real progress a call made. This makes batch migration **idempotent and safe to retry**.

---

## Step 3 — `complete_migration(admin)`

```rust
pub fn complete_migration(env: Env, admin: Address) -> Result<(), Error>
```

- Authorizes/pause-checks `admin`, then requires an active migration
  (missing status or `in_progress == false` → `Error::Basic(BasicError::MigrationNotStarted)` **106**).
- Refuses to close while work is outstanding: if `migrated_count < total_count` it fails with
  `Error::Basic(BasicError::MigrationNotStarted)` (**106**). Migrate every record first.
- On success it sets `in_progress = false`, `completed_at = Some(env.ledger().timestamp())`, and
  writes `ConfigKey::SchemaVersion = MIGRATION_TARGET_SCHEMA_VERSION` (**2**), so
  `get_schema_version()` now returns `2` and `begin_migration` will reject any later attempt with
  `SchemaAlreadyAtTarget` (**114**).

---

## Verifying completion with `get_migration_status()`

```rust
pub fn get_migration_status(env: Env) -> MigrationStatus
```

Read-only, no auth. If no migration has ever been started it returns a zeroed status, so the call is
always safe to make.

```rust
pub struct MigrationStatus {
    pub in_progress: bool,
    pub migrated_count: u64,
    pub total_count: u64,
    pub started_at: u64,
    pub completed_at: Option<u64>,
}
```

| Field | How to read it |
| --- | --- |
| `in_progress` | `true` between `begin_migration` and `complete_migration` |
| `migrated_count` | Records successfully migrated so far |
| `total_count` | Escrow counter snapshot taken by `begin_migration` |
| `started_at` | Ledger timestamp of the `begin_migration` call |
| `completed_at` | `Some(timestamp)` once `complete_migration` succeeded, else `None` |

**A migration is complete exactly when all three hold:**

1. `get_migration_status().in_progress == false`
2. `get_migration_status().completed_at.is_some()`
3. `get_schema_version() == 2`

During the migration window the invariant `migrated_count <= total_count` holds, and
`complete_migration` is the only thing that moves `migrated_count` to the `total_count` finish line.

---

## What happens to normal escrow operations during a migration

The migration window is **not** a full contract pause, and only one normal-path entry point is gated:

| Operation | While `in_progress == true` |
| --- | --- |
| `create_escrow(...)` | **Blocked** — returns `Error::Basic(BasicError::ContractPaused)` (**103**) |
| Existing-escrow operations (release, dispute, clawback, sub-accounts, swap, health queries, …) | Unaffected — they do not consult `MigrationStatus` |

So a migration window blocks *new* escrow creation only, while existing escrows keep flowing. Once
`complete_migration` flips `in_progress` back to `false`, `create_escrow` succeeds again — a new
escrow created right after completion is stored in the migrated (version 2) shape, so there is no need
to migrate it.

If you need to stop *everything* (not just creation) for an upgrade, use the pause controls instead:
a global pause makes all four migration entry points fail with `ContractPaused` too.

---

## Error reference

| Error | Code | Raised when |
| --- | --- | --- |
| `Error::Basic(BasicError::NotAnAdmin)` | 101 | Caller is not in the multisig admin set |
| `Error::Basic(BasicError::ContractPaused)` | 103 | Contract globally paused, the function is individually paused, or `create_escrow` is called during an active migration |
| `Error::Basic(BasicError::MigrationNotStarted)` | 106 | `begin_migration` after a completed migration; any step with no active migration; `complete_migration` with `migrated_count < total_count` |
| `Error::Basic(BasicError::AlreadyMigrated)` | 107 | `migrate_escrow` called twice for the same `escrow_id` (batch mode skips these instead) |
| `Error::Basic(BasicError::SchemaAlreadyAtTarget)` | 114 | `begin_migration` when `get_schema_version() >= 2` |
| `Error::Escrow(EscrowError::NotFound)` | 200 | `migrate_escrow` for an `escrow_id` that has no stored escrow (batch mode skips these instead) |

---

## Operator walkthrough

```rust
// 1. Open the window (admin signs).
client.begin_migration(&admin);

// 2. Confirm the window and how much work there is.
let status = client.get_migration_status();
assert!(status.in_progress);
assert_eq!(status.migrated_count, 0);
// status.total_count is the number of escrows to migrate.

// 3. Migrate in batches until migrated_count reaches total_count.
//    The return value is the number actually migrated, so log it per call.
let migrated: u32 = client.migrate_escrow_batch(&admin, &ids);

// A single straggler can be migrated on its own.
client.migrate_escrow(&admin, &straggler_id);

// 4. Verify before closing — complete_migration fails while work is outstanding.
let status = client.get_migration_status();
assert!(status.migrated_count == status.total_count);

// 5. Close the window and stamp the schema version.
client.complete_migration(&admin);

// 6. Final verification.
let status = client.get_migration_status();
assert!(!status.in_progress);
assert!(status.completed_at.is_some());
assert_eq!(client.get_schema_version(), 2);
```

---

## See also

- [Storage Schema Versioning](./STORAGE_VERSIONING.md) — the repository-wide versioning convention
- [`contracts/escrow/src/lib.rs`](../contracts/escrow/src/lib.rs) — migration implementation
- [`contracts/escrow/src/migration_test.rs`](../contracts/escrow/src/migration_test.rs) — behavioural tests for every step
- [Escrow contract README](../contracts/escrow/README.md) — the rest of the escrow API
