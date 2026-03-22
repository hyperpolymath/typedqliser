#![forbid(unsafe_code)]
// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
pub mod abi;
pub mod codegen;
pub mod manifest;
pub mod plugins;
pub use manifest::{load_manifest, Manifest};

/// Check all queries in a project against the type safety levels.
pub fn check(manifest_path: &str) -> anyhow::Result<Vec<codegen::CheckResult>> {
    let m = load_manifest(manifest_path)?;
    codegen::check_queries(&m, None, false)
}
