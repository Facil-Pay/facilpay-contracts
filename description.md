Implement batch payment gas optimization with aggregated transfers.

This change introduces a new optimized batch payment function that groups `BatchPaymentEntry` values by `(token, merchant)` and executes a single `token.transfer()` per group. Aggregated transfers reduce ledger operations and gas usage while preserving per-entry results and failure isolation.

Added behavior:
- Groups same-token entries for a merchant into one aggregated transfer.
- Keeps `BatchResult` output identical to the existing batch payment interface.
- Maintains isolation so one entry failure does not rollback unrelated entries.
- Adds a preview function to estimate batch gas usage.

Issue: 128
