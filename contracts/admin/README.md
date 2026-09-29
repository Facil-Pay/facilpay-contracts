# Admin Contract

Part of the [FacilPay smart contracts](../../README.md) suite on Stellar/Soroban.

## Purpose

The admin contract gates privileged operations across the other contracts. It acts as a central authority that can coordinate emergency actions — such as pausing and unpausing the payment, escrow, and refund contracts — in a single Soroban call, and can update the child contract addresses it points at.

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
- The pauser is the only role allowed to call `emergency_pause_all` and `emergency_unpause_all`.
- The pauser address must also be an admin/multisig member of each child contract. Otherwise the child contract's `pause_contract` / `unpause_contract` call rejects the pauser and the emergency call fails.

### Permission Checks

Every privileged function performs two authorization checks:

1. **Authentication** — the caller must pass Soroban's `require_auth()` for the supplied address.
2. **Authorization** — the caller's address must match the stored role address exactly (admin or pauser, depending on the function). A mismatch returns `Error::Unauthorized`.

### Privileged Operations

| Function | Description | Role Required |
|---|---|---|
| `initialize(admin, pauser, payment_contract, escrow_contract, refund_contract)` | Configures the contract with the admin, pauser, and child contract addresses. | Admin (sets both roles) |
| `emergency_pause_all(pauser, reason)` | Pauses the payment, escrow, and refund contracts in one call. | Pauser |
| `emergency_unpause_all(pauser, reason)` | Unpauses the payment, escrow, and refund contracts in one call. | Pauser |
| `set_payment_contract(admin, payment_contract)` | Updates the stored payment contract address. | Admin |
| `set_escrow_contract(admin, escrow_contract)` | Updates the stored escrow contract address. | Admin |
| `set_refund_contract(admin, refund_contract)` | Updates the stored refund contract address. | Admin |

### Read-Only Operations

| Function | Description | Returns |
|---|---|---|
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
