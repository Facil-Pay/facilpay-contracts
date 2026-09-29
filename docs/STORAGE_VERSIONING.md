# Storage Schema Versioning

FacilPay Soroban smart contracts (`contracts/payment`, `contracts/refund`, `contracts/escrow`) implement an explicit storage schema versioning convention. This allows deployed contracts to track their data storage layout version on-chain and perform state migrations as stored data structures evolve over time.

---

## 📐 How Storage Schema Versions Are Tracked

Every contract tracks its schema version in instance storage under a dedicated storage key (`ConfigKey::SchemaVersion` or `SystemKey::SchemaVersion`).

### Key Functions

1. **`get_schema_version(env: Env) -> u32`**
   - Returns the current schema version number stored in contract instance storage.
   - Defaults to `1` (`INITIAL_SCHEMA_VERSION`) if no custom version has been written yet.

2. **`migrate_schema(env: Env, admin: Address, target_version: u32) -> Result<(), Error>`**
   - Authorized admin-only function that updates the contract schema version to `target_version`.
   - Returns an error (`SchemaAlreadyAtTarget`) if the current stored version is already greater than or equal to `target_version`.

---

## 🛠️ Contributor Workflow: Changing Stored Data Shapes

When modifying an existing stored data structure (such as adding fields to a struct, modifying enum variants, or restructuring storage keys), contributors must adhere to the following workflow:

1. **Assess Breaking Changes**:
   - Determine if the change breaks backwards compatibility with existing on-chain data.
   - Adding non-optional fields or re-interpreting existing byte encodings requires a schema migration.

2. **Define Migration Logic**:
   - Update `migrate_schema()` in the relevant contract (e.g., [`contracts/payment/src/lib.rs`](../contracts/payment/src/lib.rs) or [`contracts/refund/src/lib.rs`](../contracts/refund/src/lib.rs)) to handle reading historical data shapes and writing upgraded data structures.

3. **Increment Target Schema Version**:
   - Ensure contract calls specify the new target version integer (`target_version > current_version`).

4. **Add & Update Unit Tests**:
   - Create or update contract tests to verify that:
     - `get_schema_version()` starts at `1` after contract `initialize()`.
     - `migrate_schema()` successfully increments the version when called by an authorized admin.
     - Calling `migrate_schema()` with a target version `<= current_version` fails with `SchemaAlreadyAtTarget`.

---

## 🧪 Reference Examples

The repository includes explicit tests demonstrating schema version initialization and migration enforcement:

- **Payment Contract**: [`contracts/payment/src/schema_version_test.rs`](../contracts/payment/src/schema_version_test.rs)
- **Refund Contract**: [`contracts/refund/src/schema_version_test.rs`](../contracts/refund/src/schema_version_test.rs)

### Example Test Pattern

```rust
#[test]
fn test_schema_version_initialized_to_one() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    assert_eq!(client.get_schema_version(), 1);
}

#[test]
fn test_migrate_schema_rejects_already_at_target() {
    let env = Env::default();
    env.mock_all_auths();
    let contract_id = env.register(RefundContract, ());
    let client = RefundContractClient::new(&env, &contract_id);
    let admin = Address::generate(&env);
    client.initialize(&admin);

    client.migrate_schema(&admin, &2);
    assert_eq!(client.get_schema_version(), 2);

    let result = client.try_migrate_schema(&admin, &2);
    assert_eq!(result, Err(Ok(Error::Ext(ExtError::SchemaAlreadyAtTarget))));
}
```

---

## 🗄️ Payment Contract Schema v2: Storage Tiers (Issue #648)

Payment contract schema **v2** moves every per-record and per-user key out of instance storage.
Instance storage is one ledger entry that is read in full on every call and has a hard size limit,
so it now only holds small, bounded, contract-global state (config, pause state, global counters).
Everything else — payments, subscriptions, requests, channels, proposals, and all per-customer and
per-merchant data — is stored in persistent storage, one entry per key.

- `DataKey::is_instance()` in [`contracts/payment/src/lib.rs`](../contracts/payment/src/lib.rs) decides the tier for each key.
  New keys must be classified there: anything keyed by a record id or an address belongs in persistent storage.
- Contract code reads and writes through `env.store()`, which routes each key to its tier and extends the
  TTL of persistent entries on every read and write (threshold ~30 days, extended to ~90 days).
- `initialize()` now writes schema version `2`. Contracts that predate schema tracking still report `1`.

**Migration path.** v2 ships before mainnet, so there is no deployed data to migrate and no automatic
migration is provided. A v1 deployment that already holds records in instance storage must be
redeployed: upgrading its code in place would leave those records in instance storage, where v2 no
longer looks for them. See the [payment README](../contracts/payment/README.md#storage-layout-issue-648).

---

## ⬆️ Upgrading Code In Place: the Refund Contract

`contracts/refund` exposes `upgrade(admin, new_wasm_hash)` (Issue #643), which swaps the contract's WASM
while keeping its address and storage. When the new code changes stored data shapes, pair the two calls:

1. Upload the new WASM and call `upgrade(admin, new_wasm_hash)`.
2. Call `migrate_schema(admin, target_version)` — this now runs the new code, which performs the migration.

See [Contract Upgrades](../contracts/refund/README.md#contract-upgrades) in the refund README.

---

## 🛟 Multi-Step Migrations: the Escrow Contract

The single-call `migrate_schema(admin, target_version)` pattern above is the convention for
`contracts/payment` and `contracts/refund`, which migrate their version marker only.

`contracts/escrow` goes further: because it holds one record per escrow, it upgrades those records
with a batched, resumable flow instead of a single call. The three steps are:

1. `begin_migration(admin)` — snapshot `total_count` and open the migration window.
2. `migrate_escrow(admin, escrow_id)` / `migrate_escrow_batch(admin, escrow_ids)` — re-write records in
   the new shape and mark them migrated.
3. `complete_migration(admin)` — close the window once every record is migrated and write
   `ConfigKey::SchemaVersion = 2`.

`get_migration_status()` reports `in_progress`, `migrated_count`, `total_count`, `started_at` and
`completed_at`, and is how you verify completion. While the window is open, `create_escrow` is
rejected with `ContractPaused` (**103**); all other escrow operations continue normally.

👉 **Full operator runbook: [Escrow Storage Migration](./ESCROW_MIGRATION.md)**
