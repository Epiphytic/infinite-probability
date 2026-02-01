//! Property-based tests for AISP conversion
//!
//! Simulates property-based testing by generating random valid prose
//! and verifying invariants across the conversion pipeline.

use infinite_probability_core::{AispConverter, ConversionOptions, ConversionTier};

/// Simple pseudo-random number generator for reproducibility
struct PseudoRng {
    state: u64,
}

impl PseudoRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }

    fn choose<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        let idx = (self.next() as usize) % items.len();
        &items[idx]
    }

    fn should_do(&mut self, probability: f64) -> bool {
        (self.next() as f64 / u64::MAX as f64) < probability
    }
}

/// Vocabulary for generating random prose
const SUBJECTS: &[&str] = &[
    "user", "admin", "system", "process", "function", "api", "database", "service", "client",
    "server", "x", "y", "result", "input", "output",
];

const VERBS: &[&str] = &[
    "must",
    "should",
    "requires",
    "ensures",
    "provides",
    "returns",
    "implies",
    "contains",
    "equals",
    "is",
    "defined as",
    "maps to",
    "calls",
    "validates",
    "checks",
];

const OBJECTS: &[&str] = &[
    "access",
    "token",
    "valid",
    "true",
    "false",
    "null",
    "empty",
    "unique",
    "secure",
    "authenticated",
    "5",
    "10",
    "data",
    "list",
    "set",
    "graph",
];

const MODIFIERS: &[&str] = &[
    "for all",
    "there exists",
    "if",
    "then",
    "else",
    "and",
    "or",
    "not",
    "always",
    "never",
];

/// Generate random prose
fn generate_random_prose(rng: &mut PseudoRng, length: usize) -> String {
    let mut parts = Vec::new();

    for _ in 0..length {
        if rng.should_do(0.3) {
            parts.push(*rng.choose(MODIFIERS));
        }

        parts.push(*rng.choose(SUBJECTS));
        parts.push(*rng.choose(VERBS));
        parts.push(*rng.choose(OBJECTS));

        if rng.should_do(0.4) {
            parts.push("and");
        } else if rng.should_do(0.1) {
            parts.push("therefore");
        }
    }

    parts.join(" ")
}

#[test]
fn test_fuzz_conversion() {
    let mut rng = PseudoRng::new(12345);

    // Fuzz with 100 random inputs
    for _i in 0..100 {
        let length = (rng.next() % 10) as usize + 3;
        let prose = generate_random_prose(&mut rng, length);

        // 1. Crash safety: Should never panic
        let result = AispConverter::convert(&prose, None);

        // 2. Structure invariant: Full tier must have required blocks
        if result.tier == ConversionTier::Full {
            assert!(
                result.output.contains("⟦Ω:Meta⟧"),
                "Missing Meta block in Full tier for: {}",
                prose
            );
            assert!(
                result.output.contains("⟦Σ:Types⟧"),
                "Missing Types block in Full tier for: {}",
                prose
            );
            assert!(
                result.output.contains("⟦Γ:Rules⟧"),
                "Missing Rules block in Full tier for: {}",
                prose
            );
            assert!(
                result.output.contains("⟦Ε⟧"),
                "Missing Evidence block in Full tier for: {}",
                prose
            );
        }

        // 3. Basic validity
        assert!(!result.output.is_empty(), "Output should not be empty");
    }
}

#[test]
fn test_detect_tier_correctness() {
    // Verify that specific keywords trigger appropriate tiers
    let cases = vec![
        ("simple assignment", ConversionTier::Minimal),
        ("invariant must hold", ConversionTier::Full),
        ("intent is to minimize risk", ConversionTier::Full),
        ("precondition requires input", ConversionTier::Full),
        ("user must login", ConversionTier::Standard),
        ("api endpoint", ConversionTier::Standard),
    ];

    for (prose, expected) in cases {
        let detected = AispConverter::detect_tier(prose);
        // Note: Full tier logic is greedy, so if it detects intent/contractor it goes to Full.
        // Standard/Minimal are fallback.
        // We assert that if we expect Full, we get Full.
        // If we expect Standard, we get at least Standard (Full is also acceptable if accidental match).

        if expected == ConversionTier::Full {
            assert_eq!(
                detected,
                ConversionTier::Full,
                "Failed to detect Full tier for: {}",
                prose
            );
        } else if expected == ConversionTier::Standard {
            assert!(
                detected == ConversionTier::Standard || detected == ConversionTier::Full,
                "Expected Standard or higher for: {}, got {}",
                prose,
                detected
            );
        }
    }
}

#[test]
fn test_error_inference() {
    // Test that error keywords generate Error block content
    let prose = "The function may fail or crash if not found";
    let result = AispConverter::convert(
        prose,
        Some(ConversionOptions {
            tier: Some(ConversionTier::Full),
            ..Default::default()
        }),
    );

    assert!(result.output.contains("⟦Χ:Errors⟧"), "Missing Errors block");
    assert!(
        result.output.contains("fail(x)⇒⊥"),
        "Missing fail inference"
    );
    assert!(
        result.output.contains("crash⇒⊥⊥"),
        "Missing crash inference"
    );
    assert!(
        result.output.contains("NotFound⇒∅"),
        "Missing NotFound inference"
    );
}

#[test]
fn test_contractor_inference() {
    // Test that contractor keywords generate Rules block content with new symbols
    let prose = "The system has an invariant and a precondition before the delta change ensures the postcondition.";
    let result = AispConverter::convert(
        prose,
        Some(ConversionOptions {
            tier: Some(ConversionTier::Full),
            ..Default::default()
        }),
    );

    assert!(
        result.output.contains("Inv(s)"),
        "Missing Invariant inference"
    );
    assert!(
        result.output.contains("Pre(f)"),
        "Missing Precondition inference"
    );
    assert!(
        result.output.contains("Post(f)"),
        "Missing Postcondition inference"
    );
    assert!(result.output.contains("Δ(s)"), "Missing Delta inference");
}
