#![cfg(test)]

extern crate std;
use std::path::Path;
use std::string::{String, ToString};
use std::vec::Vec;

const GLOSSARY_CONTENT: &str = include_str!("../../../docs/GLOSSARY.md");

const REQUIRED_NEW_TERMS: &[&str] = &[
    "Arbitration",
    "Basis Points (bps)",
    "Circuit Breaker",
    "Dunning",
    "Instance Storage",
    "Payment Channel",
    "Persistent Storage",
    "Proration",
    "Refund Voucher",
];

const PREVIOUS_TERMS: &[&str] = &[
    "Clawback",
    "Escrow",
    "Finality Delay",
    "Horizon",
    "Multisig",
    "Reason Code",
    "Spend Limits",
    "Sub-Account",
    "Threshold",
    "WASM",
];

fn extract_glossary_headings(content: &str) -> Vec<String> {
    content
        .lines()
        .filter(|line| line.starts_with("## "))
        .map(|line| line.trim_start_matches("## ").trim().to_string())
        .collect()
}

#[test]
fn test_glossary_contains_all_required_and_legacy_terms() {
    let headings = extract_glossary_headings(GLOSSARY_CONTENT);

    for term in REQUIRED_NEW_TERMS {
        assert!(
            headings.iter().any(|h| h == *term),
            "Glossary missing newly required term: {}",
            term
        );
    }

    for term in PREVIOUS_TERMS {
        assert!(
            headings.iter().any(|h| h == *term),
            "Glossary missing previous term: {}",
            term
        );
    }

    assert!(
        headings.len() >= REQUIRED_NEW_TERMS.len() + PREVIOUS_TERMS.len(),
        "Expected at least {} terms in glossary, found {}",
        REQUIRED_NEW_TERMS.len() + PREVIOUS_TERMS.len(),
        headings.len()
    );
}

#[test]
fn test_glossary_alphabetical_order() {
    let headings = extract_glossary_headings(GLOSSARY_CONTENT);
    assert!(!headings.is_empty(), "Glossary must have entries");

    for i in 0..headings.len() - 1 {
        let current = &headings[i];
        let next = &headings[i + 1];

        let current_key = current.to_lowercase();
        let next_key = next.to_lowercase();

        assert!(
            current_key < next_key,
            "Glossary entries not in alphabetical order: '{}' should come after '{}'",
            next,
            current
        );
    }
}

#[test]
fn test_each_entry_has_definition_and_readme_link() {
    let sections: Vec<&str> = GLOSSARY_CONTENT.split("\n## ").collect();
    // First section is the title "# Glossary..."
    assert!(sections.len() > 1, "Must contain ## sections");

    for section in &sections[1..] {
        let lines: Vec<&str> = section.lines().collect();
        let title = lines[0].trim();

        // Check definition exists (first non-empty paragraph)
        let body_lines: Vec<&str> = lines
            .iter()
            .skip(1)
            .map(|l| l.trim())
            .filter(|l| !l.is_empty() && !l.starts_with("---"))
            .collect();

        assert!(
            !body_lines.is_empty(),
            "Entry '{}' is missing body content/definition",
            title
        );

        let first_paragraph = body_lines[0];
        // Definition should be 1-3 sentences
        let sentence_count = first_paragraph
            .split(|c| c == '.' || c == '!' || c == '?')
            .filter(|s| !s.trim().is_empty())
            .count();
        assert!(
            (1..=5).contains(&sentence_count),
            "Entry '{}' definition has unexpected sentence count: {} (paragraph: '{}')",
            title,
            sentence_count,
            first_paragraph
        );

        // Check that a markdown link exists in the entry
        assert!(
            section.contains("](") && section.contains(".md"),
            "Entry '{}' must link to a relevant README or documentation section",
            title
        );
    }
}

#[test]
fn test_glossary_relative_file_links_exist() {
    // Collect all markdown link targets in GLOSSARY_CONTENT
    // Links in docs/GLOSSARY.md are relative to docs/
    let mut offset = 0;
    while let Some(open_paren) = GLOSSARY_CONTENT[offset..].find("](") {
        let start = offset + open_paren + 2;
        if let Some(close_paren) = GLOSSARY_CONTENT[start..].find(')') {
            let link = &GLOSSARY_CONTENT[start..start + close_paren];
            offset = start + close_paren + 1;

            // Only check relative file links (ending in .md or .md#...)
            if link.contains(".md") {
                let file_path_str = link.split('#').next().unwrap();
                let manifest_dir = env!("CARGO_MANIFEST_DIR");
                // contracts/payment -> manifest_dir
                // docs/ is at manifest_dir/../../docs
                let docs_dir = Path::new(manifest_dir).join("../../docs");
                let target_file = docs_dir.join(file_path_str);

                assert!(
                    target_file.exists(),
                    "Link '{}' in docs/GLOSSARY.md points to non-existent file: {:?}",
                    link,
                    target_file
                );
            }
        } else {
            break;
        }
    }
}

#[test]
fn test_glossary_exact_function_and_error_names() {
    // Verify that key contract function names and error names are correctly mentioned
    let required_symbols = &[
        // Arbitration
        "escalate_to_arbitration",
        "cast_arbitration_vote",
        "close_arbitration_case",
        "trigger_arbitration_timeout",
        "CoreError::RefundNotFound",
        "ExtError::QuorumNotReached",
        // Circuit Breaker
        "set_circuit_breaker_config",
        "get_circuit_breaker_state",
        "reset_circuit_breaker",
        "check_circuit_breaker",
        "CircuitBreakerTripped",
        // Dunning
        "set_dunning_config",
        "retry_failed_payment",
        "resolve_dunning",
        "SubscriptionError::NotInDunning",
        "SubscriptionError::RetryTooEarly",
        // Payment Channel
        "open_channel",
        "settle_channel",
        "close_channel_expired",
        "FeatureError::ChannelNotFound",
        "FeatureError::ChannelExpired",
        // Proration
        "resume_subscription",
        "set_subscription_proration",
        "SubscriptionResumedProrated",
        // Refund Voucher
        "issue_refund_voucher",
        "redeem_refund_voucher",
        "ExtError::VoucherNotFound",
        "ExtError::VoucherExpired",
        // Storage
        "get_schema_version",
        "migrate_schema",
        "attach_invoice",
    ];

    for symbol in required_symbols {
        assert!(
            GLOSSARY_CONTENT.contains(symbol),
            "Glossary should reference exact function/error name '{}'",
            symbol
        );
    }
}
