use anyhow::{Result, bail};
use serde_json::Value;

/// Parse a JSON string and validate it's an object
///
/// # Arguments
/// - `json_str`: The JSON string to parse
/// - `command_name`: Name of the command (for error messages)
///
/// # Returns
/// The parsed JSON object or an error
pub fn parse_json_object(
    json_str: &str,
    command_name: &str,
) -> Result<serde_json::Map<String, Value>> {
    let json_value: Value = serde_json::from_str(json_str)?;

    if let Some(obj) = json_value.as_object() {
        Ok(obj.clone())
    } else {
        bail!("{} requires a JSON object", command_name)
    }
}

/// Parse a JSON string into a Value
///
/// Simple wrapper around serde_json::from_str for consistency
///
/// # Arguments
/// - `json_str`: The JSON string to parse
///
/// # Returns
/// The parsed JSON value or an error
pub fn parse_json_value(json_str: &str) -> Result<Value> {
    Ok(serde_json::from_str(json_str)?)
}

/// Format a JSON value with pretty printing
pub fn format_json(value: &serde_json::Value) -> Result<String> {
    Ok(serde_json::to_string_pretty(value)?)
}

/// Print a JSON value with pretty formatting
pub fn print_json(value: &serde_json::Value) -> Result<()> {
    println!("{}", format_json(value)?);
    Ok(())
}
