//! npm Dependency Manager - Security types
//!
//! Contains NpmSecurityConfig, VulnerabilitySeverity, NpmAuditResult,
//! and NpmDependency structs.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

use anyhow::{bail, Result};
use semver::Version;

const MAX_NPM_PACKAGE_NAME_LEN: usize = 214;
const MAX_NPM_VERSION_SPEC_LEN: usize = 128;

/// npm dependency specification
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct NpmDependency {
    pub name: String,
    pub version: String,
}

impl NpmDependency {
    pub fn new(name: String, version: String) -> Self {
        Self { name, version }
    }

    pub fn validate(&self) -> Result<()> {
        validate_registry_package_name(&self.name)?;
        validate_registry_version_range(&self.version)
    }
}

pub(super) fn validate_registry_package_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > MAX_NPM_PACKAGE_NAME_LEN {
        bail!("npm package name must be 1-{MAX_NPM_PACKAGE_NAME_LEN} bytes");
    }
    if name.trim() != name || name.bytes().any(|byte| byte.is_ascii_uppercase()) {
        bail!("npm package name must be lowercase and cannot contain whitespace");
    }

    let valid_segment = |segment: &str| {
        !segment.is_empty()
            && segment.bytes().all(|byte| {
                byte.is_ascii_lowercase() || byte.is_ascii_digit() || b"._-".contains(&byte)
            })
            && segment
                .as_bytes()
                .first()
                .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
            && segment
                .as_bytes()
                .last()
                .is_some_and(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit())
    };

    if let Some(scoped) = name.strip_prefix('@') {
        let mut parts = scoped.split('/');
        let scope = parts.next().unwrap_or_default();
        let package = parts.next().unwrap_or_default();
        if parts.next().is_some() || !valid_segment(scope) || !valid_segment(package) {
            bail!("invalid scoped npm package name '{name}'");
        }
    } else if name.contains('/') || !valid_segment(name) {
        bail!("invalid npm package name '{name}'");
    }

    Ok(())
}

pub(super) fn validate_registry_version_range(version: &str) -> Result<()> {
    if version.is_empty() || version.len() > MAX_NPM_VERSION_SPEC_LEN || version.trim() != version {
        bail!("npm version must be a non-empty exact SemVer without surrounding whitespace");
    }

    let lower = version.to_ascii_lowercase();
    const FORBIDDEN_PREFIXES: &[&str] = &[
        "file:",
        "link:",
        "git:",
        "git+",
        "http:",
        "https:",
        "workspace:",
        "npm:",
    ];
    if FORBIDDEN_PREFIXES
        .iter()
        .any(|prefix| lower.starts_with(prefix))
        || version.contains('/')
        || version.contains('\\')
        || version.contains(':')
    {
        bail!("npm dependency versions must be exact registry SemVer versions");
    }

    Version::parse(version)
        .map(|_| ())
        .map_err(|error| anyhow::anyhow!("invalid exact npm registry SemVer '{version}': {error}"))
}

/// Vulnerability severity levels
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum VulnerabilitySeverity {
    Low,
    Moderate,
    High,
    Critical,
}

impl VulnerabilitySeverity {
    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "low" => Some(Self::Low),
            "moderate" => Some(Self::Moderate),
            "high" => Some(Self::High),
            "critical" => Some(Self::Critical),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Low => "low",
            Self::Moderate => "moderate",
            Self::High => "high",
            Self::Critical => "critical",
        }
    }
}

/// Security configuration for npm dependency management
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmSecurityConfig {
    pub whitelist: HashSet<String>,
    pub enforce_version_lock: bool,
    pub enable_audit: bool,
    pub fail_on_audit_vulnerabilities: bool,
    pub max_vulnerability_severity: VulnerabilitySeverity,
}

impl Default for NpmSecurityConfig {
    fn default() -> Self {
        Self {
            whitelist: HashSet::new(),
            enforce_version_lock: true,
            enable_audit: false,
            fail_on_audit_vulnerabilities: false,
            max_vulnerability_severity: VulnerabilitySeverity::High,
        }
    }
}

/// npm audit result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NpmAuditResult {
    pub vulnerabilities: HashMap<VulnerabilitySeverity, usize>,
    pub total: usize,
    pub passed: bool,
    pub raw_output: String,
}
