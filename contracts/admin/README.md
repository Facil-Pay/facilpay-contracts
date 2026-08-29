# Admin Contract

Part of the [FacilPay smart contracts](../../README.md) suite on Stellar/Soroban.

## Purpose

The admin contract gates privileged operations across the other contracts. It acts as a central authority that can coordinate emergency actions — such as pausing the payment, escrow, and refund contracts — in a single Soroban call.

## Role & Permission Model

### Admin Address

- The admin is a single `Address` stored in contract instance storage under the `Admin` data key.
- It is set **once** during `initialize(admin, payment_contract, escrow_contract, refund_contract)` and cannot be changed after initialization.
- Calling `initialize` a second time returns `Error::AlreadyInitialized`.
- The admin address must authorize the `initialize` call via Soroban authentication (`require_auth()`).

### Permission Checks

Every privileged function performs two authorization checks:

1. **Authentication** — the caller must pass Soroban's `require_auth()` for the supplied admin address.
2. **Authorization** — the caller's address must match the stored admin address exactly. A mismatch returns `Error::Unauthorized`.

### Privileged Operations

| Function | Description | Admin Required |
|---|---|---|
| `initialize(admin, payment_contract, escrow_contract, refund_contract)` | Deploys and configures the contract with the admin and child contract addresses. | Yes (sets the admin) |
| `emergency_pause_all(admin, reason)` | Pauses the payment, escrow, and refund contracts in one call. Requires the caller to be the stored admin. | Yes |

### Error Codes

| Code | Constant | Description |
|---|---|---|
| 1 | `AlreadyInitialized` | `initialize` was called more than once. |
| 2 | `NotInitialized` | A privileged function was called before `initialize`. |
| 3 | `Unauthorized` | The caller's address does not match the stored admin. |

## Security Considerations

- The admin address should be a **multi-sig** or **governance contract** address, never a single private key, to avoid a single point of failure.
- Because `emergency_pause_all` halts all child contracts, the admin key should be treated as a high-value credential and stored securely (e.g., in a hardware wallet or threshold-signing scheme).
- There is no `transfer_admin` or `renounce_admin` function — the admin role is permanent. Review deployment scripts carefully before calling `initialize`.

---

## See Also

- [Root README](../../README.md) — architecture overview and workspace setup
- [Payment Contract](../payment/README.md)
- [Escrow Contract](../escrow/README.md)
- [Refund Contract](../refund/README.md)
