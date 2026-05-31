# TODO - Refund Analytics Upgrade

## Plan step checklist
- [ ] Update `contracts/refund/src/lib.rs`:
  - [ ] Replace existing `RefundAnalytics` struct with test-required fields
  - [ ] Add `RefundAnalyticsBucket`
  - [ ] Add new daily bucket + merchant-isolated storage keys
- [ ] Update analytics lifecycle updates:
  - [ ] In `process_refund_internal()`: update `avg_processing_time_seconds` and processed day buckets
  - [ ] In `approve_refund_internal()` and `reject_refund()`: update approved/rejected counts + totals per day buckets
  - [ ] In `escalate_to_arbitration()`: increment `total_arbitration_cases` (opened)
- [ ] Implement new query functions in `contracts/refund/src/lib.rs`:
  - [ ] `get_refund_analytics_range(env, from, to)` daily aggregation (inclusive range)
  - [ ] `get_merchant_refund_analytics(env, merchant)` merchant isolation
- [ ] Ensure compilation with existing contract bindings
- [ ] Run tests:
  - [ ] `cargo test -p refund`
  - [ ] Fix any failures and re-run

