//! Infinite Probability Core Library
//!
//! Core functionality for the Infinite Probability plugin marketplace including:
//! - AISP conversion (prose ↔ symbolic notation) via rosetta-aisp
//! - LLM fallback via rosetta-aisp-llm
//! - Configuration management
//! - Shared utilities

pub mod config;

// Re-export rosetta-aisp types
pub use rosetta_aisp::{
    AispConverter, ConversionOptions, ConversionResult, ConversionTier, RosettaStone, TokenStats,
};

// Re-export rosetta module for direct access
pub use rosetta_aisp as rosetta;

// Re-export LLM fallback from rosetta-aisp-llm
pub use rosetta_aisp_llm::{
    convert_with_fallback, ClaudeFallback, ConversionOptionsExt, LlmProvider, LlmResult,
};

pub use config::Config;

/// Prelude for convenient imports
pub mod prelude {
    pub use crate::config::Config;
    pub use anyhow::Result;
    pub use rosetta_aisp::{
        AispConverter, ConversionOptions, ConversionResult, ConversionTier, RosettaStone,
        TokenStats,
    };
    pub use rosetta_aisp_llm::{
        convert_with_fallback, ClaudeFallback, ConversionOptionsExt, LlmProvider, LlmResult,
    };
}
