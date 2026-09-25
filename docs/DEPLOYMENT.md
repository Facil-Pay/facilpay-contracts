# Deploying Smart Contracts to Stellar Testnet

This guide walks you through building, setting up identities, funding accounts, deploying, initializing, and verifying all four FacilPay smart contracts (`admin`, `payments`, `escrow`, and `refund`) on Stellar Testnet using the **Stellar CLI**.

---

## 1. Prerequisites

Before beginning, ensure your development environment has the following installed:

1. **Rust Toolchain**: Rust version 1.74.0 or later.
   ```bash
   curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
   ```

2. **WASM Compilation Targets**: The `wasm32v1-none` target is required for modern Soroban v23+ contracts, alongside `wasm32-unknown-unknown`.
   ```bash
   rustup target add wasm32v1-none
   rustup target add wasm32-unknown-unknown
   ```

3. **Stellar CLI**:
   ```bash
   cargo install --locked stellar-cli --features opt
   ```

4. **Verify Stellar CLI Installation**:
   ```bash
   stellar --version
   ```

5. **Build Tools**: Standard C build chain (`make`, `gcc`, `git`).

---

## 2. Build

Build all smart contracts in the workspace using `make` or `stellar contract build`:

```bash
# From repository root
make
```

Or build directly via Stellar CLI:

```bash
stellar contract build
```

This compiles each workspace contract crate and outputs four WebAssembly (`.wasm`) artifacts into `target/wasm32v1-none/release/`:

| Contract Crate | WASM Artifact Name | Output Path |
| --- | --- | --- |
| `admin` | `admin.wasm` | `target/wasm32v1-none/release/admin.wasm` |
| `escrow` | `escrow.wasm` | `target/wasm32v1-none/release/escrow.wasm` |
| `payments` | `payments.wasm` | `target/wasm32v1-none/release/payments.wasm` |
| `refund` | `refund.wasm` | `target/wasm32v1-none/release/refund.wasm` |

### WASM Size Verification

Soroban limits contract WASM sizes to 262,144 bytes (256 KB). The Makefile includes size enforcement:

```bash
make check-size
```

If any WASM file exceeds 256 KB, optimization settings in `Cargo.toml` (`opt-level = "z"`, `lto = true`) must be verified.

---

## 3. Identity & Funding

### 3.1 Configure Testnet Network

Add the Stellar Testnet RPC endpoint and network passphrase to your Stellar CLI global configuration:

```bash
stellar network add --global testnet \
  --rpc-url https://soroban-testnet.stellar.org:443 \
  --network-passphrase "Test SDF Network ; July 2015"
```

### 3.2 Generate Keypairs

Generate two identity keypairs:
- `deployer`: Deployer account and contract administrator.
- `pauser`: Pauser account authorized for emergency pause/unpause operations on the admin contract.

```bash
stellar keys generate deployer --network testnet
stellar keys generate pauser --network testnet
```

Query the public key addresses for each identity:

```bash
stellar keys address deployer
stellar keys address pauser
```

### 3.3 Fund Accounts via Friendbot

Fund both keypair accounts on Testnet using Stellar Friendbot:

```bash
stellar keys fund deployer --network testnet
stellar keys fund pauser --network testnet
```

Alternatively, request testnet XLM via `curl`:

```bash
curl "https://friendbot.stellar.org/?addr=$(stellar keys address deployer)"
curl "https://friendbot.stellar.org/?addr=$(stellar keys address pauser)"
```

---

## 4. Deploy

Deploy the compiled WASM binaries to Stellar Testnet and export their assigned contract IDs.

### 4.1 Deploy Payment Contract (`payments.wasm`)

```bash
export PAYMENT_ID=$(stellar contract deploy \
  --wasm target/wasm32v1-none/release/payments.wasm \
  --source deployer \
  --network testnet)

echo "Payment Contract ID: $PAYMENT_ID"
```

### 4.2 Deploy Escrow Contract (`escrow.wasm`)

```bash
export ESCROW_ID=$(stellar contract deploy \
  --wasm target/wasm32v1-none/release/escrow.wasm \
  --source deployer \
  --network testnet)

echo "Escrow Contract ID: $ESCROW_ID"
```

### 4.3 Deploy Refund Contract (`refund.wasm`)

```bash
export REFUND_ID=$(stellar contract deploy \
  --wasm target/wasm32v1-none/release/refund.wasm \
  --source deployer \
  --network testnet)

echo "Refund Contract ID: $REFUND_ID"
```

### 4.4 Deploy Admin Contract (`admin.wasm`)

```bash
export ADMIN_ID=$(stellar contract deploy \
  --wasm target/wasm32v1-none/release/admin.wasm \
  --source deployer \
  --network testnet)

echo "Admin Contract ID: $ADMIN_ID"
```

---

## 5. Initialize

Contracts must be initialized in a strict order due to cross-contract linkage requirements:

1. **Payment Contract**: `initialize(admin)`
2. **Escrow Contract**: `initialize(admin)`
3. **Refund Contract**: `initialize(admin)`
4. **Link Refund to Payment Contract**: `set_payment_contract_address(admin, payment_contract)`
5. **Admin Contract**: `initialize(admin, pauser, payment_contract, escrow_contract, refund_contract)`

---

### Step 5.1: Initialize Payment Contract

- **Function**: `initialize`
- **Parameters**: `admin: Address`

```bash
stellar contract invoke \
  --id $PAYMENT_ID \
  --source deployer \
  --network testnet \
  -- \
  initialize \
  --admin $(stellar keys address deployer)
```

---

### Step 5.2: Initialize Escrow Contract

- **Function**: `initialize`
- **Parameters**: `admin: Address`

```bash
stellar contract invoke \
  --id $ESCROW_ID \
  --source deployer \
  --network testnet \
  -- \
  initialize \
  --admin $(stellar keys address deployer)
```

---

### Step 5.3: Initialize Refund Contract

- **Function**: `initialize`
- **Parameters**: `admin: Address`

```bash
stellar contract invoke \
  --id $REFUND_ID \
  --source deployer \
  --network testnet \
  -- \
  initialize \
  --admin $(stellar keys address deployer)
```

---

### Step 5.4: Link Payment Contract Address in Refund Contract

- **Function**: `set_payment_contract_address`
- **Parameters**:
  - `admin: Address`
  - `payment_contract: Address`

```bash
stellar contract invoke \
  --id $REFUND_ID \
  --source deployer \
  --network testnet \
  -- \
  set_payment_contract_address \
  --admin $(stellar keys address deployer) \
  --payment_contract $PAYMENT_ID
```

---

### Step 5.5: Initialize Admin Contract

- **Function**: `initialize`
- **Parameters**:
  - `admin: Address`
  - `pauser: Address`
  - `payment_contract: Address`
  - `escrow_contract: Address`
  - `refund_contract: Address`

```bash
stellar contract invoke \
  --id $ADMIN_ID \
  --source deployer \
  --network testnet \
  -- \
  initialize \
  --admin $(stellar keys address deployer) \
  --pauser $(stellar keys address pauser) \
  --payment_contract $PAYMENT_ID \
  --escrow_contract $ESCROW_ID \
  --refund_contract $REFUND_ID
```

---

## 6. Verify

Verify on-chain that each contract was deployed and initialized successfully using read-only query calls.

### 6.1 Verify Payment Contract

Query `get_schema_version`:

```bash
stellar contract invoke \
  --id $PAYMENT_ID \
  --network testnet \
  -- \
  get_schema_version
```

**Expected Output**: `1`

---

### 6.2 Verify Escrow Contract

Query `get_schema_version`:

```bash
stellar contract invoke \
  --id $ESCROW_ID \
  --network testnet \
  -- \
  get_schema_version
```

**Expected Output**: `1`

---

### 6.3 Verify Refund Contract

Query `get_schema_version` and `get_payment_contract_address`:

```bash
stellar contract invoke \
  --id $REFUND_ID \
  --network testnet \
  -- \
  get_schema_version

stellar contract invoke \
  --id $REFUND_ID \
  --network testnet \
  -- \
  get_payment_contract_address
```

**Expected Output for schema version**: `1`
**Expected Output for payment contract address**: `"$PAYMENT_ID"`

---

### 6.4 Verify Admin Contract

Verify initialization status by attempting to invoke `initialize` a second time (confirming `Error::AlreadyInitialized` protection):

```bash
stellar contract invoke \
  --id $ADMIN_ID \
  --source deployer \
  --network testnet \
  -- \
  initialize \
  --admin $(stellar keys address deployer) \
  --pauser $(stellar keys address pauser) \
  --payment_contract $PAYMENT_ID \
  --escrow_contract $ESCROW_ID \
  --refund_contract $REFUND_ID
```

**Expected Output**: Reverts with `Error(Contract, #1)` (`AlreadyInitialized`).

---

## 7. Troubleshooting

| Error Code / Message | Root Cause | Solution |
| --- | --- | --- |
| `Error(Contract, #1)` / `AlreadyInitialized` | `initialize` was called on an already initialized contract. | The contract is already set up. Do not invoke `initialize` again. |
| `Error(Contract, #2)` / `NotInitialized` | A privileged admin function was invoked before `initialize`. | Execute `initialize` on the contract first. |
| `Error(Contract, #3)` / `Unauthorized` | The `--source` keypair does not match the configured admin address or missing authorization. | Confirm `--source` matches the `--admin` address passed during initialization. |
| `can't find crate for core` / `wasm32v1-none` target missing | Rust toolchain missing the `wasm32v1-none` compilation target. | Run `rustup target add wasm32v1-none`. |
| `HostError: Error(Budget, ExceededLimit)` | Contract uncompressed WASM exceeds size limits or execution budget exceeded. | Run `make check-size` and verify release profile optimizations (`opt-level = "z"`). |
| `AccountNotFound` / `TxFailed` | Deployer account is unfunded on Testnet. | Run `stellar keys fund deployer --network testnet`. |
| Cross-Contract invocation error (`EscrowBridgeFailed` / `501`) | Payment or Refund contract address misconfigured or missing. | Ensure `set_payment_contract_address` was invoked on the refund contract with the correct `$PAYMENT_ID`. |
