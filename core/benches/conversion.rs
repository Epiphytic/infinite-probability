//! Conversion benchmarks

use criterion::{black_box, criterion_group, criterion_main, Criterion};
use infinite_probability_core::prelude::*;

fn benchmark_minimal_conversion(c: &mut Criterion) {
    let prose = "Define x as 5 and for all y in S, x equals y";

    c.bench_function("minimal_conversion", |b| {
        b.iter(|| {
            AispConverter::convert(
                black_box(prose),
                Some(ConversionOptions {
                    tier: Some(ConversionTier::Minimal),
                    ..Default::default()
                }),
            )
        });
    });
}

fn benchmark_standard_conversion(c: &mut Criterion) {
    let prose = "The user must authenticate before accessing the API endpoint";

    c.bench_function("standard_conversion", |b| {
        b.iter(|| {
            AispConverter::convert(
                black_box(prose),
                Some(ConversionOptions {
                    tier: Some(ConversionTier::Standard),
                    ..Default::default()
                }),
            )
        });
    });
}

fn benchmark_full_conversion(c: &mut Criterion) {
    let prose = "Define a type User with id and name. All users must have valid credentials to prove access.";

    c.bench_function("full_conversion", |b| {
        b.iter(|| {
            AispConverter::convert(
                black_box(prose),
                Some(ConversionOptions {
                    tier: Some(ConversionTier::Full),
                    ..Default::default()
                }),
            )
        });
    });
}

fn benchmark_tier_detection(c: &mut Criterion) {
    let samples = vec![
        "Define x as 5",
        "The user must provide valid authentication to access the API",
        "Define a type User and verify all users are valid with proof",
    ];

    c.bench_function("tier_detection", |b| {
        b.iter(|| {
            for sample in &samples {
                AispConverter::detect_tier(black_box(sample));
            }
        });
    });
}

fn benchmark_rosetta_lookup(c: &mut Criterion) {
    use rosetta_aisp::prose_to_symbol;

    let patterns = vec!["for all", "exists", "implies", "boolean", "unknown_word"];

    c.bench_function("rosetta_lookup", |b| {
        b.iter(|| {
            for pattern in &patterns {
                prose_to_symbol(black_box(pattern));
            }
        });
    });
}

criterion_group!(
    benches,
    benchmark_minimal_conversion,
    benchmark_standard_conversion,
    benchmark_full_conversion,
    benchmark_tier_detection,
    benchmark_rosetta_lookup,
);
criterion_main!(benches);
