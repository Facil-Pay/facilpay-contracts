# FacilPay Smart Contracts

Stellar-based smart contracts for FacilPay. Secure, auditable, and transparent payment infrastructure.

## 🏗️ Architecture

FacilPay uses Soroban smart contracts on Stellar for:
- **Payment Processing**: Accept and lock crypto payments
- **Settlement**: Convert and transfer to merchants in USDC
- **Escrow**: Hold funds during dispute periods
- **Refunds**: Process customer refunds

## 📋 Prerequisites

- Rust 1.74.0 or later
- Stellar CLI (`stellar` command)
- Soroban SDK

### Installation

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install Stellar CLI
cargo install --locked stellar-cli --features opt

# Add wasm target
rustup target add wasm32-unknown-unknown
```

## 🚀 Quick Start

### Build All Contracts

```bash
# From root directory
make
```

### Run Tests

```bash
# Test all contracts in workspace
cargo test --workspace

# Test specific contract
cargo test -p escrow
cargo test -p payment
cargo test -p refund
```

## 📂 Contract Overview

### Payment Contract (`contracts/payment`)

Handles payment creation and processing:
- `create_payment()` - Customer initiates payment
- `complete_payment()` - Admin releases to merchant
- `refund_payment()` - Admin refunds to customer
- `get_payment()` - Query payment details

### Escrow Contract (`contracts/escrow`)

Manages fund holding and disputes:
- `create_escrow()` - Lock funds
- `release_escrow()` - Release to merchant
- `dispute_escrow()` - Handle disputes

### Refund Contract (`contracts/refund`)

Processes refund requests:
- `request_refund()` - Merchant initiates
- `approve_refund()` - Admin approves
- `process_refund()` - Execute refund
- `get_refunds_by_reason_code()` - Filter refunds by structured reason code with pagination
- `get_reason_code_analytics()` - Count refunds by reason code (sorted by frequency)

#### Refund Reason Code Migration (Breaking)

`request_refund()` now requires a `reason_code: RefundReasonCode` argument in addition to free-text `reason`.

- Old call shape: `request_refund(..., reason, payment_created_at)`
- New call shape: `request_refund(..., reason, reason_code, payment_created_at)`

Recommended migration path:
1. Update all callers to pass a concrete enum value (`ProductDefect`, `NonDelivery`, `DuplicateCharge`, `Unauthorized`, `CustomerRequest`, `Other`).
2. For unknown/legacy flows, pass `Other` first and backfill specific codes in your upstream app logic.
3. If upgrading a deployed instance with existing data, plan a storage/data migration for historical refunds before reading them as the new `Refund` shape.

## 🔄 Development Workflow

1. Fork the repo
2. Create a feature branch (`git checkout -b feature/amazing-feature`)
3. Write tests for your changes
4. Ensure all tests pass (`cargo test --workspace`)
5. Commit your changes (`git commit -m 'Add amazing feature'`)
6. Push to your branch (`git push origin feature/amazing-feature`)
7. Open a Pull Request

## 🔗 Links

<!-- - [Website](https://facilpay.com) coming soon -->
<!-- - [Documentation](https://docs.facilpay.com) coming soon -->
- Telegram: https://t.me/+afM9uh7GGtVkYmZk
- [API Repository](https://github.com/facilpay/facilpay-api)
- [SDK Repository](https://github.com/facilpay/facilpay-sdk)
