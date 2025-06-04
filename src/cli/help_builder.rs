//! Dynamic help text builder for CLI

use super::cargo_detector::CargoDetector;

/// Get the detected Bevy app name if available
pub fn get_detected_app() -> Option<String> {
    match CargoDetector::new() {
        Ok(detector) => {
            // First try to get the default binary
            if let Some(default_binary) = detector.get_default_binary() {
                if default_binary.is_bevy_app {
                    return Some(default_binary.name);
                }
            }

            // Otherwise look for any Bevy app
            let bevy_apps = detector.find_bevy_apps();
            bevy_apps.first().map(|app| app.name.clone())
        }
        Err(_) => None,
    }
}
