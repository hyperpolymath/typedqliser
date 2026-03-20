// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
//
// Query language plugin system.
// Each supported query language implements the QueryLanguagePlugin trait.

pub mod sql;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// A query extracted from a source file, with its location.
#[derive(Debug, Clone)]
pub struct LocatedQuery {
    /// The query text.
    pub query: String,
    /// Source file path.
    pub file: String,
    /// Line number in the source file (1-indexed).
    pub line: usize,
}

/// Database schema: tables, columns, types.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Schema {
    pub tables: Vec<TableDef>,
}

/// A table definition with columns.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TableDef {
    pub name: String,
    pub columns: Vec<ColumnDef>,
}

/// A column definition with name, type, and nullability.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    #[serde(rename = "type")]
    pub col_type: String,
    #[serde(default)]
    pub nullable: bool,
    #[serde(default)]
    pub primary_key: bool,
}

/// Trait that all query language plugins implement.
pub trait QueryLanguagePlugin: Send + Sync {
    /// Plugin name (e.g., "sql", "graphql").
    fn name(&self) -> &str;

    /// Parse a query string. Returns Ok(()) if syntactically valid (Level 1).
    fn parse_check(&self, query: &str) -> Result<()>;

    /// Check that all referenced tables/columns exist in the schema (Level 2).
    fn schema_check(&self, query: &str, schema: &Schema) -> Result<Vec<SchemaIssue>>;

    /// Check type compatibility of operations (Level 3).
    fn type_check(&self, query: &str, schema: &Schema) -> Result<Vec<TypeIssue>>;

    /// Check null safety — all nullable paths handled (Level 4).
    fn null_check(&self, query: &str, schema: &Schema) -> Result<Vec<NullIssue>>;
}

/// A schema binding issue (Level 2 failure).
#[derive(Debug, Clone)]
pub struct SchemaIssue {
    pub message: String,
}

/// A type compatibility issue (Level 3 failure).
#[derive(Debug, Clone)]
pub struct TypeIssue {
    pub message: String,
}

/// A null safety issue (Level 4 failure).
#[derive(Debug, Clone)]
pub struct NullIssue {
    pub message: String,
    pub column: String,
}

/// Get the plugin for a given language name.
pub fn get_plugin(language: &str) -> Result<Box<dyn QueryLanguagePlugin>> {
    match language {
        "sql" | "postgresql" | "mysql" | "sqlite" => Ok(Box::new(sql::SqlPlugin::new(language))),
        _ => anyhow::bail!(
            "Unsupported language: {}. Run `typedqliser languages` to see options.",
            language
        ),
    }
}

/// Print supported query languages.
pub fn print_languages() {
    println!("=== Supported Query Languages ===");
    println!();
    println!("  {:12} {:40} {:10} {}", "Language", "Databases", "Max Level", "Status");
    println!("  {:12} {:40} {:10} {}", "--------", "---------", "---------", "------");
    println!("  {:12} {:40} {:10} {}", "sql",     "PostgreSQL, MySQL, SQLite, MSSQL",    "6",  "Active (MVP)");
    println!("  {:12} {:40} {:10} {}", "graphql", "Any GraphQL API",                      "6",  "Planned");
    println!("  {:12} {:40} {:10} {}", "cypher",  "Neo4j, Memgraph, Apache AGE",          "4",  "Planned");
    println!("  {:12} {:40} {:10} {}", "sparql",  "Any RDF triplestore",                  "4",  "Planned");
    println!("  {:12} {:40} {:10} {}", "vql",     "VeriSimDB",                             "10", "Planned (via VQL-UT)");
    println!("  {:12} {:40} {:10} {}", "kql",     "Azure Data Explorer / Kusto",           "4",  "Planned");
    println!("  {:12} {:40} {:10} {}", "gql",     "ISO GQL databases",                     "4",  "Planned");
    println!("  {:12} {:40} {:10} {}", "fqldt",   "Lithoglyph",                            "10", "Planned");
    println!("  {:12} {:40} {:10} {}", "custom",  "User-defined grammar",                  "var","Plugin API");
}
