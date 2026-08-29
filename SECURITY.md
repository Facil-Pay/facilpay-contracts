# Security Policy

This document describes the security vulnerability disclosure process for the FacilPay smart contracts repository.

## Reporting a Vulnerability

If you discover a security vulnerability in this repository, please report it responsibly and do not disclose the issue publicly until a fix has been released.

### Reporting Contact

Email: **security@facilpay.com**

**Response time:** We aim to respond to vulnerability reports within 48 hours.

### What to Include

When reporting a vulnerability, please provide:

1. **Description** — A clear summary of the vulnerability and its potential impact.
2. **Affected Component** — Which contract(s) and function(s) are affected (e.g., payment contract's `complete_payment()`, refund contract's `escalate_to_arbitration()`).
3. **Severity** — Your assessment of severity (Critical, High, Medium, Low).
4. **Steps to Reproduce** — Clear steps or proof-of-concept code demonstrating the issue (without triggering any real damage).
5. **Suggested Fix** — If you have recommendations for remediation, we welcome them.
6. **Contact Information** — Your name, email, and preferred contact method.

## Supported Versions

Security updates are provided for the following versions:

| Version | Status | Support Until |
|---------|--------|----------------|
| Latest main branch | Active | Ongoing |
| Previous tagged release | Limited | 6 months after latest release |
| Older releases | Unsupported | Not applicable |

We recommend always running the latest version to receive security fixes and feature improvements.

## Disclosure Timeline

Once a vulnerability is reported:

1. **Acknowledgment (48 hours)** — We confirm receipt and provide an initial assessment.
2. **Investigation (1–2 weeks)** — Our security team reproduces and analyzes the issue.
3. **Fix Development (1–4 weeks depending on severity)** — A patch is developed and tested.
4. **Pre-release Notification (3–5 days before release)** — We notify downstream projects (API repo, SDK repo) of the fix.
5. **Public Disclosure (on release)** — The fix is released publicly; we issue a security advisory and credit the researcher.

### Expedited Timeline for Critical Issues

Critical vulnerabilities (e.g., fund loss, contract compromise) are prioritized:

- **Fix Target:** 1 week
- **Release Target:** 2 weeks from initial report
- **Pre-release notification:** 5 days before release

## What We Consider a Vulnerability

### In Scope

- Unauthorized fund transfer or lockup
- Contract state corruption or bypass of access controls
- Integer overflow/underflow leading to incorrect balances
- Cross-contract call failures that leave escrow in an unsafe state
- Signature/authentication bypass
- Reentrancy or state machine violations
- Cryptographic weaknesses
- Event emission failures that break off-chain indexers

### Out of Scope

- Issues in documentation or comments (report via pull request instead)
- Speculative issues without proof-of-concept
- Performance issues that don't affect correctness
- Vulnerabilities in dependent libraries (report to the library maintainers)
- Social engineering or phishing attacks

## Bug Bounty

At this time, we do not operate a formal bug bounty program. However, we deeply appreciate security researchers who help us improve the safety of our contracts. Researchers who responsibly disclose vulnerabilities will be:

- **Credited** in our security advisory and this repository
- **Acknowledged** in release notes
- **Considered for future bug bounty programs**

## Security Best Practices for Integrators

If you are integrating these contracts into your application:

1. **Keep Updated** — Subscribe to releases and apply security patches promptly.
2. **Audit Dependent Contracts** — These contracts rely on external escrow and token contracts; ensure those are audited and trusted.
3. **Monitor Events** — Use the documented Soroban events to verify contract behavior off-chain.
4. **Test Edge Cases** — Particularly around refund limits, multi-sig governance, and arbitration timeouts.
5. **Rate Limiting** — Enable the built-in rate limiting and fraud detection features.
6. **Access Controls** — Use multi-sig governance for sensitive operations like admin upgrades.

## Public Disclosure

Once a fix is released, we will:

1. Publish a security advisory in this repository
2. Tag the release with a security indicator
3. Document the issue in the CHANGELOG.md
4. Credit the researcher (unless they request anonymity)

## Contact & Questions

For security-related inquiries other than vulnerability reports, please contact:

**security@facilpay.com**

For general questions or feature requests, see the root [README.md](README.md) for community links.

---

**Last Updated:** 2026-07-29

For the most up-to-date security information, visit the [FacilPay security page](https://facilpay.com/security).
