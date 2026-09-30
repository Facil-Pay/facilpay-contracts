# Admin Contract

Part of the [FacilPay smart contracts](../../README.md) suite on Stellar/Soroban.

## Purpose

The admin contract gates privileged operations across the other contracts. It acts as a central authority that can coordinate emergency actions — such as pausing and unpausing the payment, escrow, and refund contracts in a single Soroban call, or pausing one named function on one child contract — and can update the child contract addresses it points at.

## Role & Permission Model

The contract defines two distinct roles, both stored in instance storage and both set **once** during `initialize`:

### Admin Address

- The admin is a single `Address` stored in contract instance storage under the `Admin` data key.
- It is set during `initialize(admin, pauser, payment_contract, escrow_contract, refund_contract)` and cannot be changed after initialization.
- Calling `initialize` a second time returns `Error::AlreadyInitialized`.
- The admin address must authorize the `initialize` call via Soroban authentication (`require_auth()`).
- The admin is the only role allowed to update the child contract addresses (`set_payment_contract`, `set_escrow_contract`, `set_refund_contract`).

### Pauser Address

- The pauser is a single `Address` stored in contract instance storage under the `Pauser` data key.
- It is set during `initialize` alongside the admin and cannot be changed after initialization.
- The pauser is the only role allowed to call `emergency_pause_all`, `emergency_unpause_all`, `emergency_pause_function` and `emergency_unpause_function`.
- The pauser address must also be an admin/multisig member of each child contract. Otherwise the child contract's `pause_contract` / `unpause_contract` / `pause_function` / `unpause_function` call rejects the pauser and the emergency call fails.

### Permission Checks

Every privileged function performs two authorization checks:

1. **Authentication** — the caller must pass Soroban's `require_auth()` for the supplied address.
2. **Authorization** — the caller's address must match the stored role address exactly (admin or pauser, depending on the function). A mismatch returns `Error::Unauthorized`.

### Privileged Operations

| Function | Description | Role Required |
|---|---|---|
| `initialize(admin, pauser, payment_contract, escrow_contract, refund_contract)` | Configures the contract with the admin, pauser, and child contract addresses. | Admin (sets both roles) |
| `emergency_pause_all(pauser, reason)` | Pauses the payment, escrow, and refund contracts in one call. | Pauser |
| `emergency_unpause_all(pauser)` | Unpauses the payment, escrow, and refund contracts in one call. | Pauser |
| `emergency_pause_function(pauser, target, function_name, reason)` | Pauses one function on the targeted child contract only. | Pauser |
| `emergency_unpause_function(pauser, target, function_name)` | Unpauses one function on the targeted child contract only. | Pauser |
| `set_payment_contract(admin, payment_contract)` | Updates the stored payment contract address. | Admin |
| `set_escrow_contract(admin, escrow_contract)` | Updates the stored escrow contract address. | Admin |
| `set_refund_contract(admin, refund_contract)` | Updates the stored refund contract address. | Admin |

### Error Codes

| Code | Constant | Description |
|---|---|---|
| 1 | `AlreadyInitialized` | `initialize` was called more than once. |
| 2 | `NotInitialized` | A privileged function was called before `initialize`. |
| 3 | `Unauthorized` | The caller's address does not match the stored admin or pauser. |
| 4 | `InvalidFunctionName` | `emergency_pause_function` / `emergency_unpause_function` was called with an empty `function_name`. |

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

## Security Considerations

- Both the admin and pauser addresses should be **multi-sig** or **governance contract** addresses, never a single private key, to avoid a single point of failure.
- Because `emergency_pause_all` halts all child contracts, the pauser key should be treated as a high-value credential and stored securely (e.g., in a hardware wallet or threshold-signing scheme).
- The pauser must be an admin/multisig member of each child contract, otherwise the child's `pause_contract` call rejects it.
- The child contract addresses are **mutable**: the admin can point the contract at new payment, escrow, or refund contracts via `set_payment_contract`, `set_escrow_contract`, and `set_refund_contract`. Review these setters carefully, as they redirect all future emergency calls.
- There is no `transfer_admin` or `renounce_admin` function — the admin and pauser roles are permanent. Review deployment scripts carefully before calling `initialize`.

---

## See Also

- [Root README](../../README.md) — architecture overview and workspace setup
- [Payment Contract](../payment/README.md)
- [Escrow Contract](../escrow/README.md)
- [Refund Contract](../refund/README.md)
