Implement rate limiting per merchant with configurable thresholds.

This change introduces merchant-specific rate limiting support in the payment contract. It adds a new `MerchantRateLimit` struct and management functions for setting, querying, and resetting merchant limits. Merchants with custom limits will be evaluated against their own thresholds, while merchants without a custom configuration will continue to use the global `RateLimitConfig`.

Key behaviors:
- Merchant limits override global limits when configured.
- The rate limit window resets automatically after one hour.
- `check_rate_limit()` can preview whether a merchant and amount would exceed limits without consuming the allowance.

Issue: 110