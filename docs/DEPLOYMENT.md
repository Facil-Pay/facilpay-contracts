# Deploying Smart Contracts to Stellar Testnet

This guide walks you through building, setting up identities, funding accounts, and deploying all four FacilPay smart contracts (`admin`, `payment`, `escrow`, and `refund`) to the Stellar Testnet using the **Stellar CLI**.

---

## Prerequisites

Before beginning, ensure you have the following installed:

1. **Rust & `wasm32` Target**:
   ```bash
   rustup target add wasm32-unknown-unknown
