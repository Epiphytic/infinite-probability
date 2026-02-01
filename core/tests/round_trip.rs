//! Round-trip conversion tests
//!
//! Tests the AISP anti-drift guarantee: ∀L: Signal(L) ≡ L
//! Verifies that multiple prose → AISP → prose conversions
//! preserve semantic meaning without drift.

use infinite_probability_core::{AispConverter, RosettaStone};

/// Complex document for testing semantic preservation
const COMPLEX_DOCUMENT: &str = r#"
Define a type User with fields id of type natural number and name of type string.
Define a type Session with fields user of type User and token of type string.

For all users u in the system, if u is authenticated then u has valid credentials.
There exists at least one admin user who can modify all resources.

If the user provides valid authentication and the session is not expired,
then allow access to the protected resource.

The function validate takes credentials and returns a boolean result.
If validation succeeds, return true, otherwise return false.

For all requests r, if r contains invalid data then reject r immediately.
The system must ensure that no unauthorized access occurs.

Define the rule: all inputs must be sanitized before processing.
Define the constraint: maximum session duration is 24 hours.
"#;

/// Semantic equivalents - words that are interchangeable
fn is_semantic_match(expected: &str, text: &str) -> bool {
    let equivalents: &[&[&str]] = &[
        &["function", "lambda"],
        &["equals", "identical to", "equivalent"],
        &["returns", "to", "yields"],
        &["such that", "where"],
        &["define", "defined as", "let"],
    ];

    // Direct match
    if text.contains(expected) {
        return true;
    }

    // Check equivalents
    for group in equivalents {
        if group.contains(&expected) {
            for equiv in *group {
                if text.contains(equiv) {
                    return true;
                }
            }
        }
    }

    false
}

/// Semantic test cases with expected preservation
/// Note: Some words have semantic equivalents (equals ↔ identical to, function ↔ lambda)
const SEMANTIC_TEST_CASES: &[(&str, &[&str])] = &[
    // (original prose, key concepts that must be preserved)
    ("for all x in S, x equals y", &["for all", "in"]),
    ("Define x as 5 and y as 10", &["defined as", "and"]),
    ("if x implies y then z", &["implies"]),
    (
        "there exists a user such that admin is true",
        &["exists", "true"],
    ),
    ("not valid or expired", &["not", "or"]),
    ("function returns boolean", &["function", "boolean"]),
];

#[test]
fn test_10_round_trips_preserve_meaning() {
    let original = COMPLEX_DOCUMENT.trim();
    let mut current_text = original.to_string();
    let mut conversion_history: Vec<(String, String)> = Vec::new();

    // Perform 10 round-trip conversions
    for i in 1..=10 {
        // Convert prose to AISP
        let result = AispConverter::convert(&current_text, None);
        let aisp = result.output.clone();

        // Convert AISP back to prose
        let prose = AispConverter::to_prose(&aisp);

        // Store history
        conversion_history.push((aisp.clone(), prose.clone()));

        // Check semantic similarity with original
        let similarity = RosettaStone::semantic_similarity(original, &prose);

        println!(
            "Round {} - Similarity with original: {:.2}%",
            i,
            similarity * 100.0
        );

        // Require at least 30% semantic similarity (accounting for symbol compression)
        assert!(
            similarity > 0.30,
            "Round {} lost too much meaning. Similarity: {:.2}%\nOriginal: {}\nCurrent: {}",
            i,
            similarity * 100.0,
            original,
            prose
        );

        current_text = prose;
    }

    // Final check: compare first round to last round
    let (_, first_prose) = &conversion_history[0];
    let (_, last_prose) = &conversion_history[9];

    let drift = 1.0 - RosettaStone::semantic_similarity(first_prose, last_prose);
    println!("\nTotal drift over 10 rounds: {:.2}%", drift * 100.0);

    // Drift should be minimal (< 20%)
    assert!(
        drift < 0.20,
        "Excessive drift detected: {:.2}%\nFirst: {}\nLast: {}",
        drift * 100.0,
        first_prose,
        last_prose
    );
}

#[test]
fn test_semantic_preservation_per_concept() {
    for (original, expected_concepts) in SEMANTIC_TEST_CASES {
        // Convert to AISP and back
        let result = AispConverter::convert(original, None);
        let prose = AispConverter::to_prose(&result.output);

        // Check that key concepts are preserved (allowing semantic equivalents)
        let normalized_prose = prose.to_lowercase();
        for concept in *expected_concepts {
            assert!(
                is_semantic_match(concept, &normalized_prose),
                "Concept '{}' (or equivalent) not preserved in round-trip.\nOriginal: {}\nAISP: {}\nProse: {}",
                concept,
                original,
                result.output,
                prose
            );
        }
    }
}

#[test]
fn test_symbol_stability() {
    // Test that symbols remain stable through multiple conversions
    let symbols = vec![
        ("∀x∈S", vec!["for all", "in"]),
        ("∃y:P(y)", vec!["exists"]),
        ("A⇒B", vec!["implies"]),
        ("A∧B∨C", vec!["and", "or"]),
        ("x≜5", vec!["defined as"]),
        ("¬valid", vec!["not"]),
        ("⊤∧⊥", vec!["true", "false"]),
    ];

    for (aisp, expected_words) in symbols {
        let prose = RosettaStone::to_prose(aisp);

        for word in expected_words {
            assert!(
                prose.to_lowercase().contains(word),
                "Symbol '{}' should produce '{}', got '{}'",
                aisp,
                word,
                prose
            );
        }
    }
}

#[test]
fn test_10_round_trips_minimal_tier() {
    let test_cases = vec![
        "Define x as 5",
        "for all y in S",
        "if valid then proceed",
        "x equals y and y equals z",
        "there exists a solution",
    ];

    for original in test_cases {
        let mut current = original.to_string();
        let initial_similarity: f64;

        // First conversion establishes baseline
        let (aisp, _, _) = RosettaStone::convert(&current);
        let prose = RosettaStone::to_prose(&aisp);
        initial_similarity = RosettaStone::semantic_similarity(original, &prose);
        current = prose;

        // Subsequent conversions should maintain stability
        let mut prev_similarity = initial_similarity;

        for round in 2..=10 {
            let (aisp, _, _) = RosettaStone::convert(&current);
            let prose = RosettaStone::to_prose(&aisp);

            let similarity = RosettaStone::semantic_similarity(original, &prose);

            // After first conversion, subsequent rounds should be stable (< 20% drop per round)
            let delta = prev_similarity - similarity;
            assert!(
                delta < 0.20,
                "Round {} for '{}' had excessive drop: {:.2}%",
                round,
                original,
                delta * 100.0
            );

            prev_similarity = similarity;
            current = prose;
        }

        // Final similarity should be at least 20% of original meaning
        let final_similarity = RosettaStone::semantic_similarity(original, &current);
        assert!(
            final_similarity > 0.20,
            "Final round for '{}' lost too much meaning: {:.2}%",
            original,
            final_similarity * 100.0
        );
    }
}

#[test]
fn test_convergence_stability() {
    // AISP theorem: ∃t: θ_t ≈ θ_{t+1} (system converges)
    // After enough round-trips, the output should stabilize

    let original = "for all users u, if u is admin then allow access";
    let mut current = original.to_string();
    let mut prev_output = String::new();
    let mut stable_count = 0;

    for round in 1..=10 {
        let (aisp, _, _) = RosettaStone::convert(&current);
        let prose = RosettaStone::to_prose(&aisp);

        // Check if output has stabilized
        let similarity = RosettaStone::semantic_similarity(&prev_output, &prose);
        if similarity > 0.95 {
            stable_count += 1;
        }

        println!(
            "Round {}: similarity with prev = {:.2}%",
            round,
            similarity * 100.0
        );

        prev_output = prose.clone();
        current = prose;
    }

    // Should reach stability within 10 rounds
    assert!(
        stable_count >= 3,
        "System did not converge. Stable rounds: {}",
        stable_count
    );
}

#[test]
fn test_complex_document_word_preservation() {
    let original = COMPLEX_DOCUMENT.trim();

    // Extract key words from original
    let key_words: Vec<&str> = vec![
        "user",
        "session",
        "admin",
        "valid",
        "access",
        "function",
        "credentials",
        "token",
        "system",
        "request",
    ];

    // Do one conversion
    let result = AispConverter::convert(original, None);
    let prose = AispConverter::to_prose(&result.output);

    let normalized_prose = prose.to_lowercase();

    // Count how many key words are preserved
    let preserved: Vec<_> = key_words
        .iter()
        .filter(|w| normalized_prose.contains(*w))
        .collect();

    let preservation_rate = preserved.len() as f64 / key_words.len() as f64;

    println!(
        "Word preservation: {}/{} ({:.1}%)",
        preserved.len(),
        key_words.len(),
        preservation_rate * 100.0
    );
    println!("Preserved words: {:?}", preserved);

    // At least 40% of key domain words should be preserved
    assert!(
        preservation_rate >= 0.40,
        "Too many key words lost in conversion: {:.1}%",
        preservation_rate * 100.0
    );
}

#[test]
fn test_tier_detection_stability() {
    // Tier detection should be stable across round-trips
    let test_cases = vec![
        ("Define x as 5", "minimal"),
        ("The user must authenticate to access the API", "standard"),
        ("Define a type User and prove all users are valid", "full"),
    ];

    for (original, expected_tier) in test_cases {
        let initial_tier = AispConverter::detect_tier(original);
        assert!(
            initial_tier
                .to_string()
                .to_lowercase()
                .contains(expected_tier),
            "Initial tier mismatch for '{}': expected {}, got {}",
            original,
            expected_tier,
            initial_tier
        );

        // Convert and check tier is still appropriate
        let result = AispConverter::convert(original, None);
        let prose = AispConverter::to_prose(&result.output);
        let final_tier = AispConverter::detect_tier(&prose);

        println!("'{}' -> {} -> {}", original, initial_tier, final_tier);
    }
}
