// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
//
// Query language plugin system.

/// Print supported query languages.
pub fn print_languages() {
    println!("=== Supported Query Languages ===");
    println!();
    println!("  {:12} {:40} {:10} {}", "Language", "Databases", "Max Level", "Status");
    println!("  {:12} {:40} {:10} {}", "--------", "---------", "---------", "------");
    println!("  {:12} {:40} {:10} {}", "sql",     "PostgreSQL, MySQL, SQLite, MSSQL",    "6",  "Priority");
    println!("  {:12} {:40} {:10} {}", "graphql", "Any GraphQL API",                      "6",  "Priority");
    println!("  {:12} {:40} {:10} {}", "cypher",  "Neo4j, Memgraph, Apache AGE",          "4",  "Planned");
    println!("  {:12} {:40} {:10} {}", "sparql",  "Any RDF triplestore",                  "4",  "Planned");
    println!("  {:12} {:40} {:10} {}", "vql",     "VeriSimDB",                             "10", "Planned (via VQL-UT)");
    println!("  {:12} {:40} {:10} {}", "kql",     "Azure Data Explorer / Kusto",           "4",  "Planned");
    println!("  {:12} {:40} {:10} {}", "gql",     "ISO GQL databases",                     "4",  "Planned");
    println!("  {:12} {:40} {:10} {}", "fqldt",   "Lithoglyph",                            "10", "Planned");
    println!("  {:12} {:40} {:10} {}", "custom",  "User-defined grammar",                  "var","Plugin API");
}
