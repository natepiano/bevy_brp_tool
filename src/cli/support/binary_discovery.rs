use std::path::{Path, PathBuf};

use anyhow::Result;

/// Find a binary in the workspace using the target directory from cargo metadata
pub fn find_workspace_binary_with_target_dir(
    name: &str,
    target_dir: &Path,
    profile: Option<&str>,
) -> Result<PathBuf> {
    // If the name contains a path separator, treat it as a path (either relative or absolute)
    if name.contains('/') || name.contains('\\') {
        let path = PathBuf::from(name);
        if path.exists() {
            return Ok(path);
        } else {
            anyhow::bail!("App binary not found at specified path: {:?}", path);
        }
    }

    // Use the specified profile or default to "debug"
    let profile = profile.unwrap_or("debug");

    // Validate profile name (basic check for valid directory names)
    if profile.contains('/') || profile.contains('\\') || profile.contains('\0') {
        anyhow::bail!(
            "Invalid profile name '{}': profile names cannot contain path separators",
            profile
        );
    }

    // Use the target directory from cargo metadata to locate the binary
    let binary_path = target_dir.join(profile).join(name);

    // Check if binary exists as-is
    if binary_path.exists() {
        return Ok(binary_path);
    }

    // On Windows, try adding .exe extension if not already present
    #[cfg(windows)]
    {
        if !name.ends_with(".exe") {
            let binary_path_exe = target_dir.join(profile).join(format!("{}.exe", name));
            if binary_path_exe.exists() {
                return Ok(binary_path_exe);
            }
        }
    }

    anyhow::bail!(
        "App binary '{}' not found in target directory: {}\n\
         Searched in:\n\
         - {}\n\
         Try building the app with 'cargo build --profile {}' first.",
        name,
        target_dir.display(),
        target_dir.join(profile).join(name).display(),
        profile
    )
}
