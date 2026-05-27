# feat(refund): Implement refund notification system with on-chain event hooks

## Summary

This PR implements a comprehensive notification hook system for the refund contract, enabling external contracts to register callbacks for refund status change events. This enables composable integrations where other contracts can react to refund lifecycle events.

## Changes

### New Structures and Enums

- **`RefundEventType`**: Enum defining hookable events
  - `Requested` - When a refund is requested
  - `Approved` - When a refund is approved
  - `Rejected` - When a refund is rejected
  - `Processed` - When a refund is processed
  - `Escalated` - When a refund is escalated to arbitration

- **`NotificationHook`**: Structure storing hook registrations
  - `hook_id`: Unique identifier
  - `subscriber`: Contract address to notify
  - `events`: List of events to subscribe to
  - `active`: Whether the hook is active

### New Functions

- **`register_notification_hook(env, subscriber, events)`**: Register a new notification hook
  - Returns the hook ID
  - Enforces max 10 hooks per event type
  - Indexes hooks by event type and subscriber
  
- **`deregister_hook(env, subscriber, hook_id)`**: Deregister a notification hook
  - Only the subscriber can deregister their own hooks
  - Marks hook as inactive rather than deleting
  
- **`get_hooks_for_event(env, event_type)`**: Query all active hooks for an event type
  
- **`get_subscriber_hooks(env, subscriber)`**: Query all hooks for a subscriber

- **`invoke_hooks(env, event_type, refund_id)`**: Internal function to invoke registered hooks
  - Called synchronously after state transitions
  - Uses `try_invoke_contract` to isolate failures
  - Failed hooks emit `HookInvocationFailed` event but don't revert the operation

### Integration Points

Hooks are invoked after successful state transitions in:
- `request_refund` → `Requested` event
- `approve_refund_internal` → `Approved` event  
- `reject_refund` → `Rejected` event
- `process_refund_internal` → `Processed` event
- `escalate_to_arbitration` → `Escalated` event

### New Events

- **`HookRegistered`**: Emitted when a hook is registered
- **`HookDeregistered`**: Emitted when a hook is deregistered
- **`HookInvocationFailed`**: Emitted when a hook call fails (operation still succeeds)

### Error Codes

- `HookNotFound` (41): Hook ID doesn't exist
- `MaxHooksPerEventReached` (42): Cannot register more than 10 hooks per event
- `HookNotOwnedBySubscriber` (43): Subscriber doesn't own the hook

## Testing

Added comprehensive test suite (`test_notification_hooks.rs`) with 14 tests:

✅ **Registration Tests**
- `test_register_notification_hook` - Basic registration
- `test_register_multiple_hooks` - Multiple independent hooks
- `test_max_hooks_per_event` - Enforces 10 hook limit
- `test_hook_for_multiple_events` - Single hook for multiple events

✅ **Deregistration Tests**
- `test_deregister_hook` - Successful deregistration
- `test_deregister_hook_not_owner` - Authorization check
- `test_deregister_nonexistent_hook` - Error handling

✅ **Query Tests**
- `test_get_subscriber_hooks` - Query hooks by subscriber

✅ **Invocation Tests**
- `test_hook_invocation_on_refund_requested` - Requested event
- `test_hook_invocation_on_refund_approved` - Approved event
- `test_hook_invocation_on_refund_rejected` - Rejected event
- `test_hook_invocation_on_refund_processed` - Processed event

✅ **Failure Isolation Tests**
- `test_failed_hook_does_not_revert_operation` - Failed hooks don't revert
- `test_multiple_hooks_same_event` - Multiple hooks invoked correctly

All tests pass successfully.

## Additional Fixes

Fixed existing type mismatch error in fraud detection code (line 3655) where `refund_rate_bps` (u32) was being compared to `config.max_refund_rate_bps as u64`.

## Acceptance Criteria

✅ Hooks are called synchronously after state transitions via `env.try_invoke_contract()`  
✅ Failed hook calls do not revert the primary operation  
✅ Max 10 hooks per event type enforced to cap gas usage  
✅ Only the subscriber address can deregister their own hooks  
✅ Comprehensive tests for hook registration, invocation, failure isolation, and max-hook cap  

## Usage Example

```rust
// External contract registers for refund events
let mut events = Vec::new(&env);
events.push_back(RefundEventType::Approved);
events.push_back(RefundEventType::Processed);

let hook_id = refund_client.register_notification_hook(
    &my_contract_address,
    &events
);

// External contract implements the callback
#[contractimpl]
impl MyContract {
    pub fn on_refund_event(
        env: Env,
        event_type: RefundEventType,
        refund_id: u64
    ) {
        // React to refund events
        // e.g., update analytics, trigger workflows, etc.
    }
}

// Later, deregister when no longer needed
refund_client.deregister_hook(&my_contract_address, &hook_id);
```

## Gas Considerations

- Hook invocations are synchronous and add to transaction gas cost
- Max 10 hooks per event type caps worst-case gas usage
- Failed hooks are isolated using `try_invoke_contract` to prevent DoS
- Inactive hooks are filtered out during invocation to save gas

## Security Considerations

- Only hook owners can deregister their hooks (authorization enforced)
- Failed hooks cannot revert the primary refund operation
- Hook invocations are isolated to prevent malicious contracts from blocking refunds
- Max hooks per event prevents gas exhaustion attacks

Closes #144
