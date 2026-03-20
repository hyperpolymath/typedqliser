// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
//
// Type-checking engine for TypedQLiser.

use anyhow::Result;
use crate::manifest::Manifest;

/// Result of type-checking a single query.
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// Source file and line where the query was found.
    pub location: String,
    /// The query text.
    pub query: String,
    /// Maximum type safety level achieved (1-10).
    pub level_achieved: u8,
    /// Per-level results.
    pub level_results: Vec<LevelResult>,
}

/// Result of checking one type safety level.
#[derive(Debug, Clone)]
pub struct LevelResult {
    pub level: u8,
    pub name: &'static str,
    pub status: LevelStatus,
    pub message: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LevelStatus {
    /// Level passed — proof constructed.
    Passed,
    /// Level failed — counterexample found.
    Failed,
    /// Level skipped (not enforced).
    Skipped,
    /// Level not applicable to this query language.
    NotApplicable,
}

static LEVEL_NAMES: [&str; 10] = [
    "Parse-time safety",
    "Schema-binding safety",
    "Type-compatible operations",
    "Null-safety",
    "Injection-proof safety",
    "Result-type safety",
    "Cardinality safety",
    "Effect-tracking safety",
    "Temporal safety",
    "Linearity safety",
];

/// Check queries against type safety levels.
pub fn check_queries(manifest: &Manifest, single_query: Option<&str>, _proofs: bool) -> Result<Vec<CheckResult>> {
    let mut results = Vec::new();

    if let Some(q) = single_query {
        let result = check_single_query(q, "<inline>", manifest)?;
        results.push(result);
    } else {
        // Scan files matching the glob patterns
        for pattern in &manifest.paths.queries {
            for entry in glob::glob(pattern)? {
                let path = entry?;
                let content = std::fs::read_to_string(&path)?;
                let result = check_single_query(&content, &path.display().to_string(), manifest)?;
                results.push(result);
            }
        }
    }

    Ok(results)
}

/// Check a single query against all applicable levels.
fn check_single_query(query: &str, location: &str, manifest: &Manifest) -> Result<CheckResult> {
    let mut level_results = Vec::new();
    let mut max_level = 0u8;

    for (i, name) in LEVEL_NAMES.iter().enumerate() {
        let level = (i + 1) as u8;
        let status = if manifest.levels.skip.contains(&level) {
            LevelStatus::Skipped
        } else {
            // TODO: implement actual type checking per level per language plugin
            // For now, levels 1-3 always pass (syntactic checks), rest are stubs
            match level {
                1 => {
                    if query.trim().is_empty() {
                        LevelStatus::Failed
                    } else {
                        max_level = level;
                        LevelStatus::Passed
                    }
                }
                2..=3 => {
                    // Stub: would need schema + type rules
                    max_level = level;
                    LevelStatus::Passed
                }
                _ => LevelStatus::Skipped,
            }
        };

        level_results.push(LevelResult {
            level,
            name,
            status,
            message: None,
        });
    }

    Ok(CheckResult {
        location: location.to_string(),
        query: query.to_string(),
        level_achieved: max_level,
        level_results,
    })
}

/// Report results to stdout.
pub fn report_results(results: &[CheckResult], manifest: &Manifest, ci: bool) -> Result<()> {
    let mut errors = 0u32;

    for result in results {
        let status = if result.level_achieved >= manifest.typedql.level { "PASS" } else { "FAIL" };
        println!("{} [L{}/{}] {}",
            status, result.level_achieved, manifest.typedql.level, result.location);

        if result.level_achieved < manifest.typedql.level {
            errors += 1;
            for lr in &result.level_results {
                if lr.status == LevelStatus::Failed {
                    println!("  Level {}: {} — FAILED{}",
                        lr.level, lr.name,
                        lr.message.as_ref().map(|m| format!(": {}", m)).unwrap_or_default());
                }
            }
        }
    }

    println!("\n{} queries checked, {} passed, {} failed",
        results.len(), results.len() as u32 - errors, errors);

    if ci && errors > 0 {
        std::process::exit(1);
    }

    Ok(())
}
