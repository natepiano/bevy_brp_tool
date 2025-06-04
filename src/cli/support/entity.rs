//! Entity ID parsing and validation utilities
//!
//! This module provides centralized functionality for parsing and validating
//! entity IDs from command-line arguments, ensuring consistent error handling
//! and type conversion across all BRP commands that work with entities.

use anyhow::Result;

/// Parse entity ID from the first argument
pub fn parse_entity_arg(args: &[&str]) -> Result<u64> {
    args[0].parse().map_err(Into::into)
}
