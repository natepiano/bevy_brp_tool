use std::path::PathBuf;

use anyhow::Result;

use crate::cli::cargo_detector::CargoDetector;

/// Detect which Bevy app to run based on user input or auto-detection
///
/// Returns a tuple of (app_name, manifest_directory, target_directory)
pub fn detect_bevy_app(app_binary: Option<String>) -> Result<(String, PathBuf, PathBuf)> {
    let detector = CargoDetector::new().ok();

    match app_binary {
        Some(app_name) => handle_specified_app(app_name, detector),
        None => auto_detect_app(detector),
    }
}

/// Handle when user specifies an app name
fn handle_specified_app(
    app_name: String,
    detector: Option<CargoDetector>,
) -> Result<(String, PathBuf, PathBuf)> {
    if let Some(detector) = detector {
        if let Some(info) = detector
            .find_all_binaries()
            .into_iter()
            .find(|b| b.name == app_name)
        {
            if !info.is_bevy_app {
                eprintln!("Warning: '{}' does not appear to be a Bevy app", app_name);
            }
            let manifest_dir = get_manifest_dir(&info.manifest_path);
            let target_dir = detector.target_directory().to_path_buf();
            return Ok((app_name, manifest_dir, target_dir));
        }
    }

    // Fallback: use app name with current directory and guess target directory
    let current_dir = current_dir_or_dot();
    let target_dir = current_dir.join("target");
    Ok((app_name, current_dir, target_dir))
}

/// Auto-detect a Bevy app when none specified
fn auto_detect_app(detector: Option<CargoDetector>) -> Result<(String, PathBuf, PathBuf)> {
    let detector = detector.ok_or_else(|| {
        anyhow::anyhow!(
            "Could not detect cargo project information. Please specify an app with --app <name>"
        )
    })?;

    let target_dir = detector.target_directory().to_path_buf();

    // Try default binary first
    if let Some(default_binary) = detector.get_default_binary() {
        if default_binary.is_bevy_app {
            println!("Detected default Bevy app: {}", default_binary.name);
            let manifest_dir = get_manifest_dir(&default_binary.manifest_path);
            return Ok((default_binary.name, manifest_dir, target_dir));
        }
    }

    // Look for any Bevy app
    let bevy_apps = detector.find_bevy_apps();
    let app = bevy_apps.first().ok_or_else(|| {
        anyhow::anyhow!(
            "No Bevy app found in the current workspace. Please specify an app with --app <name>"
        )
    })?;

    println!("Using detected Bevy app: {}", app.name);
    let manifest_dir = get_manifest_dir(&app.manifest_path);
    Ok((app.name.clone(), manifest_dir, target_dir))
}

/// Get the directory containing the manifest file
fn get_manifest_dir(manifest_path: &std::path::Path) -> PathBuf {
    manifest_path
        .parent()
        .map(|p| p.to_path_buf())
        .unwrap_or_else(current_dir_or_dot)
}

/// Get current directory or fallback to "."
fn current_dir_or_dot() -> PathBuf {
    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
}
