# Contributing to FacilPay Smart Contracts

Thank you for your interest in contributing to FacilPay! We welcome contributions from the community to help build secure and reliable payment infrastructure on Stellar.

## 📜 Code of Conduct

This project follows the [Contributor Covenant Code of Conduct](CODE_OF_CONDUCT.md). By participating, you are expected to uphold this code. Please report unacceptable behavior to the project maintainers.

## 🚀 Getting Started

### Prerequisites

Before you begin, ensure you have:
- Rust 1.74.0 or later
- Stellar CLI installed (`cargo install --locked stellar-cli --features opt`)
- Basic understanding of Soroban smart contracts
- Git for version control

### Development Setup

1. **Fork and clone the repository**
   ```bash
   git clone https://github.com/YOUR_USERNAME/facilpay-contracts.git
   cd facilpay-contracts
   ```

2. **Add wasm target**
   ```bash
   rustup target add wasm32-unknown-unknown
   ```

3. **Build the contracts**
   ```bash
   make build
   ```

4. **Run tests**
   ```bash
   cargo test --workspace
   ```
### Makefile Targets

The root `Makefile` provides several helpful targets to streamline development. Run these using `make <target>`.

| Target | Description | When to Use |
|--------|-------------|-------------|
| `build` | Compiles the smart contracts using `stellar contract build` and checks the WASM sizes. | Use this whenever you want to compile your code and verify the output artifacts don't exceed size limits. |
| `test` | Runs the full `build` target, followed by `cargo test` to execute all tests. | Use this to verify that your contracts compile successfully and all tests pass. |
| `fmt` | Runs `cargo fmt --all` to format the workspace code. | Use this before committing to ensure your code adheres to standard Rust formatting guidelines. |
| `clean` | Runs `cargo clean` to remove the generated `target/` directory and build artifacts. | Use this to free up disk space or if you want to ensure a completely fresh build from scratch. |
| `check-size` | Checks the size of compiled `.wasm` artifacts against the `262144` bytes limit. | Runs automatically during `make build`. Use this manually if you just want to verify sizes without recompiling. |

### WASM Size Limit

Soroban networks enforce a maximum contract deployment size of **256 KB** (262,144 bytes). The `check-size` target enforces this limit to ensure contracts remain deployable on Stellar.

**Why this limit exists:** Soroban caps WASM blob size to keep ledger storage manageable and maintain network performance. A contract that exceeds it will be rejected at deployment by the Stellar network.

**If `make build` or `make check-size` fails with a size error:**

1. Check which contract exceeded the limit — the error message prints the file path and size.
2. **Build in release mode** — the default `stellar contract build` already compiles with optimizations.
3. **Strip debug symbols** — add `strip = true` to the release profile in `Cargo.toml`:
   ```toml
   [profile.release]
   strip = true
   ```
4. **Optimize for size** — add `opt-level = "z"` and `lto = true` in the release profile:
   ```toml
   [profile.release]
   opt-level = "z"
   lto = true
   ```
5. **Audit dependencies** — run `cargo bloat --crates` to identify large dependencies and consider lighter alternatives.
6. **Minimize contract exports** — only export the Soroban functions you need; avoid pulling in unused SDK features.
7. **Remove unused code** — use `cargo deadlinks` or review for dead code that the linker may still include.

After making changes, run `make build` to re-check the size.

## 🔄 Development Workflow

### 1. Create a Feature Branch

```bash
git checkout -b feature/your-feature-name
# or
git checkout -b fix/bug-description
```

Branch naming conventions:
- `feature/` - New features
- `fix/` - Bug fixes
- `refactor/` - Code refactoring
- `docs/` - Documentation updates
- `test/` - Test additions or modifications

### 2. Make Your Changes

- Write clean, well-documented code
- Follow Rust best practices and idioms
- Add inline comments for complex logic
- Update or add tests for your changes

### 3. Write Tests

All contract changes **must** include tests:

```bash
# Test specific contract
cd contracts/payment
cargo test

# Test all contracts
cargo test --workspace
```

Test requirements:
- Unit tests for all public functions
- Integration tests for cross-contract interactions
- Edge case coverage
- Error handling verification

### 4. Format Your Code

```bash
make fmt
# or
cargo fmt --all
```

### 5. Run All Checks

Before committing, ensure:
```bash
# Format check
cargo fmt --all -- --check

# Build all contracts
make build

# Run all tests
cargo test --workspace

# Check for warnings
cargo clippy --all-targets --all-features -- -D warnings
```

### 6. Commit Your Changes

Follow conventional commit format:

```bash
git commit -m "feat: add new payment validation"
git commit -m "fix: resolve escrow release issue"
git commit -m "test: add edge cases for refund processing"
git commit -m "docs: update payment contract documentation"
```

Commit message format:
- `feat:` - New feature
- `fix:` - Bug fix
- `refactor:` - Code refactoring
- `test:` - Adding or updating tests
- `docs:` - Documentation changes
- `chore:` - Maintenance tasks

### 7. Push and Create Pull Request

```bash
git push origin feature/your-feature-name
```

Then open a Pull Request on GitHub with:
- Clear description of changes
- Reference to related issues (if any)
- Screenshots/logs if applicable

## 📝 Code Style Guidelines

### Rust Conventions

- Follow the [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- Use descriptive variable and function names
- Keep functions focused and small
- Prefer explicit error handling over panics

### Soroban-Specific Guidelines

```rust
// ✅ Good: Clear function documentation
/// Creates a new payment with escrow
/// 
/// # Arguments
/// * `env` - Contract environment
/// * `from` - Sender address
/// * `amount` - Payment amount
/// 
/// # Panics
/// Panics if amount is zero
pub fn create_payment(env: Env, from: Address, amount: i128) -> u64 {
    // Implementation
}

// ❌ Avoid: Undocumented functions with unclear logic
pub fn cp(e: Env, f: Address, a: i128) -> u64 {
    // Implementation
}
```

### Testing Guidelines

```rust
#[test]
fn test_payment_creation() {
    let env = Env::default();
    // Setup test data
    let contract_id = env.register_contract(None, PaymentContract);
    
    // Execute test
    let payment_id = contract.create_payment(/* args */);
    
    // Verify results
    assert_eq!(payment_id, 1);
}

#[test]
#[should_panic(expected = "Amount must be positive")]
fn test_payment_creation_zero_amount() {
    // Test error cases
}
```

## 🔒 Security Considerations

Smart contracts handle real value. Please:

1. **Never bypass security checks** - All authorization checks are there for a reason
2. **Validate all inputs** - Check bounds, types, and authorization
3. **Test edge cases** - Zero amounts, maximum values, unauthorized access
4. **Consider reentrancy** - Ensure state updates happen before external calls
5. **Document security assumptions** - Explain trust boundaries in comments

## 🐛 Reporting Bugs

When reporting bugs, please include:

1. **Description** - Clear summary of the issue
2. **Steps to reproduce** - Detailed steps to trigger the bug
3. **Expected behavior** - What should happen
4. **Actual behavior** - What actually happens
5. **Environment** - Rust version, Stellar CLI version, OS
6. **Code samples** - Minimal reproducible example if possible

## 💡 Feature Requests

We welcome feature suggestions! Please:

1. Check existing issues to avoid duplicates
2. Clearly describe the feature and use case
3. Explain how it benefits FacilPay users
4. Consider backwards compatibility

## 📋 Pull Request Checklist

Before submitting your PR, verify:

- [ ] Code builds without errors (`make build`)
- [ ] All tests pass (`cargo test --workspace`)
- [ ] Code is formatted (`cargo fmt --all`)
- [ ] No clippy warnings (`cargo clippy`)
- [ ] New tests added for new functionality
- [ ] Documentation updated if needed
- [ ] Commit messages follow conventional format
- [ ] PR description clearly explains changes

## 🎯 Areas for Contribution

We especially welcome contributions in:

- **Testing** - Additional test coverage, edge cases, integration tests
- **Documentation** - Code comments, usage examples, tutorials
- **Security** - Security reviews, vulnerability testing
- **Optimization** - Gas optimization, performance improvements
- **Features** - New payment features, refund mechanisms

---

Thank you for helping make FacilPay better! 🙏
