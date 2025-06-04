//! Cargo metadata parser for detecting binary targets and Bevy applications

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use anyhow::{Context, Result};
use cargo_metadata::{Metadata, MetadataCommand, Package};

use super::constants::BIN_NAME;

/// Cached cargo metadata to avoid repeated expensive calls, keyed by directory
static METADATA_CACHE: OnceLock<Mutex<HashMap<PathBuf, Option<Metadata>>>> = OnceLock::new();

/// Information about a binary target
#[derive(Debug, Clone)]
pub struct BinaryInfo {
    /// Name of the binary
    pub name: String,
    /// Path to the Cargo.toml containing this binary
    pub manifest_path: PathBuf,
    /// Whether this binary depends on Bevy
    pub is_bevy_app: bool,
}

/// Detects binary targets in the current project or workspace
pub struct CargoDetector {
    metadata: Metadata,
}

impl CargoDetector {
    /// Create a new detector for the current directory
    pub fn new() -> Result<Self> {
        Self::from_path(std::env::current_dir()?)
    }

    /// Create a new detector for a specific path
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self> {
        let current_dir = path
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| path.as_ref().to_path_buf());

        // Get or initialize the cache
        let cache = METADATA_CACHE.get_or_init(|| Mutex::new(HashMap::new()));

        // Try to get cached metadata first
        let metadata = {
            let mut cache_guard = cache.lock().unwrap();
            if let Some(cached_result) = cache_guard.get(&current_dir) {
                cached_result.clone()
            } else {
                // Not in cache, execute cargo metadata
                let result = MetadataCommand::new().current_dir(&current_dir).exec().ok();

                // Cache the result (even if None)
                cache_guard.insert(current_dir.clone(), result.clone());
                result
            }
        };

        match metadata {
            Some(metadata) => Ok(Self { metadata }),
            None => {
                // If cache failed and no cached result, try one more direct execution
                let metadata = MetadataCommand::new()
                    .current_dir(&current_dir)
                    .exec()
                    .context("Failed to execute cargo metadata")?;

                Ok(Self { metadata })
            }
        }
    }

    /// Get the default binary that would be executed by `cargo run`
    pub fn get_default_binary(&self) -> Option<BinaryInfo> {
        // In a workspace, cargo run without -p will fail, so we need to check
        // if we're in a member package directory
        let current_dir = std::env::current_dir().ok()?;

        // First, try to find a package that contains the current directory
        // (we might be in a subdirectory of a package)
        let current_package = self.metadata.packages.iter().find(|pkg| {
            if let Some(pkg_dir) = pkg.manifest_path.parent() {
                current_dir.starts_with(pkg_dir)
            } else {
                false
            }
        });

        if let Some(package) = current_package {
            // Check if this package has a default-run
            if let Some(default_run) = &package.default_run {
                return self.find_binary_in_package(package, default_run);
            }

            // Otherwise, return the first binary target
            return self.get_first_binary_in_package(package);
        }

        // If not in any package directory, look for any Bevy app in the workspace
        // Prefer packages with default-run specified
        for package in &self.metadata.packages {
            if self.metadata.workspace_members.contains(&package.id)
                && self.package_depends_on_bevy(package)
            {
                if let Some(default_run) = &package.default_run {
                    return self.find_binary_in_package(package, default_run);
                }
                // If no default-run, continue looking for a better match
            }
        }

        // If no package with default-run found, return first Bevy app
        for package in &self.metadata.packages {
            if self.metadata.workspace_members.contains(&package.id)
                && self.package_depends_on_bevy(package)
            {
                return self.get_first_binary_in_package(package);
            }
        }

        // If this is a single package (not a workspace), return its binary
        if self.metadata.workspace_members.len() == 1 {
            let root_package = self
                .metadata
                .packages
                .iter()
                .find(|pkg| self.metadata.workspace_members.contains(&pkg.id))?;

            if let Some(default_run) = &root_package.default_run {
                return self.find_binary_in_package(root_package, default_run);
            }

            return self.get_first_binary_in_package(root_package);
        }

        None
    }

    /// Find all binary targets in the workspace/project
    pub fn find_all_binaries(&self) -> Vec<BinaryInfo> {
        let mut binaries = Vec::new();

        for package in &self.metadata.packages {
            // Only process workspace members
            if !self.metadata.workspace_members.contains(&package.id) {
                continue;
            }

            for target in &package.targets {
                if target.is_bin() {
                    let is_bevy_app = self.package_depends_on_bevy(package);
                    binaries.push(BinaryInfo {
                        name: target.name.clone(),
                        manifest_path: package.manifest_path.clone().into(),
                        is_bevy_app,
                    });
                }
            }
        }

        binaries
    }

    /// Find all Bevy applications in the workspace/project
    pub fn find_bevy_apps(&self) -> Vec<BinaryInfo> {
        self.find_all_binaries()
            .into_iter()
            .filter(|binary| binary.is_bevy_app && binary.name != BIN_NAME)
            .collect()
    }

    fn find_binary_in_package(&self, package: &Package, binary_name: &str) -> Option<BinaryInfo> {
        package
            .targets
            .iter()
            .find(|t| t.is_bin() && t.name == binary_name)
            .map(|_| BinaryInfo {
                name: binary_name.to_string(),
                manifest_path: package.manifest_path.clone().into(),
                is_bevy_app: self.package_depends_on_bevy(package),
            })
    }

    fn get_first_binary_in_package(&self, package: &Package) -> Option<BinaryInfo> {
        package
            .targets
            .iter()
            .find(|t| t.is_bin())
            .map(|target| BinaryInfo {
                name: target.name.clone(),
                manifest_path: package.manifest_path.clone().into(),
                is_bevy_app: self.package_depends_on_bevy(package),
            })
    }

    fn package_depends_on_bevy(&self, package: &Package) -> bool {
        // Check direct dependencies
        for dep in &package.dependencies {
            if dep.name == "bevy" {
                return true;
            }
        }

        // For workspace members, also check workspace dependencies
        if self.metadata.workspace_members.contains(&package.id) {
            // The package might use workspace dependencies
            // We need to check if bevy is referenced via .workspace = true
            // This is a bit tricky with cargo_metadata, so we'll use a heuristic:
            // If bevy is in workspace deps and the package has few direct deps,
            // it likely uses workspace deps
            if self.workspace_has_bevy() && package.dependencies.len() < 10 {
                return true;
            }
        }

        false
    }

    fn workspace_has_bevy(&self) -> bool {
        // Check if any workspace member depends on bevy
        self.metadata
            .packages
            .iter()
            .filter(|pkg| self.metadata.workspace_members.contains(&pkg.id))
            .any(|pkg| pkg.dependencies.iter().any(|dep| dep.name == "bevy"))
    }

    /// Get the target directory where binaries are built
    pub fn target_directory(&self) -> &Path {
        self.metadata.target_directory.as_ref()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detector_creation() {
        // This test will only work in a cargo project
        let detector = CargoDetector::new();
        assert!(detector.is_ok());
    }

    #[test]
    fn test_detector_info() {
        if let Ok(detector) = CargoDetector::new() {
            println!("=== Cargo Detector Test ===");

            // Test default binary
            if let Some(default_bin) = detector.get_default_binary() {
                println!("\nDefault binary:");
                println!("  Name: {}", default_bin.name);
                println!("  Manifest: {:?}", default_bin.manifest_path);
                println!("  Is Bevy app: {}", default_bin.is_bevy_app);
            } else {
                println!("\nNo default binary found");
            }

            // List all binaries
            println!("\nAll binaries:");
            for bin in detector.find_all_binaries() {
                println!("  - {} (Bevy: {})", bin.name, bin.is_bevy_app);
            }

            // List Bevy apps
            println!("\nBevy apps:");
            for app in detector.find_bevy_apps() {
                println!("  - {}", app.name);
            }
        }
    }
}
