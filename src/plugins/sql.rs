// SPDX-License-Identifier: PMPL-1.0-or-later
// Copyright (c) 2026 Jonathan D.A. Jewell (hyperpolymath) <j.d.a.jewell@open.ac.uk>
//
// SQL plugin for TypedQLiser.
// Uses sqlparser-rs to parse SQL into an AST, then checks type safety levels.

use anyhow::{Context, Result};
use sqlparser::ast::{
    BinaryOperator, Expr, Query, SelectItem, SetExpr, Statement, TableFactor, TableWithJoins, Value,
};
use sqlparser::dialect::{GenericDialect, MySqlDialect, PostgreSqlDialect, SQLiteDialect};
use sqlparser::parser::Parser;

use super::{NullIssue, QueryLanguagePlugin, Schema, SchemaIssue, TypeIssue};

/// SQL plugin supporting multiple dialects.
pub struct SqlPlugin {
    dialect_name: String,
}

impl SqlPlugin {
    pub fn new(dialect: &str) -> Self {
        Self {
            dialect_name: dialect.to_string(),
        }
    }

    /// Get the sqlparser dialect for this plugin instance.
    fn dialect(&self) -> Box<dyn sqlparser::dialect::Dialect> {
        match self.dialect_name.as_str() {
            "postgresql" => Box::new(PostgreSqlDialect {}),
            "mysql" => Box::new(MySqlDialect {}),
            "sqlite" => Box::new(SQLiteDialect {}),
            _ => Box::new(GenericDialect {}),
        }
    }

    /// Parse SQL into AST statements.
    fn parse(&self, query: &str) -> Result<Vec<Statement>> {
        let dialect = self.dialect();
        Parser::parse_sql(dialect.as_ref(), query).with_context(|| "SQL parse error")
    }

    /// Extract all table names referenced in a statement.
    fn extract_table_refs(statement: &Statement) -> Vec<String> {
        let mut tables = Vec::new();
        match statement {
            Statement::Query(query) => {
                Self::extract_tables_from_query(query, &mut tables);
            }
            Statement::Insert(insert) => {
                tables.push(insert.table_name.to_string().to_lowercase());
                if let Some(ref source) = insert.source {
                    Self::extract_tables_from_query(source.as_ref(), &mut tables);
                }
            }
            Statement::Update { table, .. } => {
                if let Some(name) = Self::table_factor_name(&table.relation) {
                    tables.push(name);
                }
            }
            Statement::Delete(delete) => match &delete.from {
                sqlparser::ast::FromTable::WithFromKeyword(twjs)
                | sqlparser::ast::FromTable::WithoutKeyword(twjs) => {
                    for twj in twjs {
                        if let Some(name) = Self::table_factor_name(&twj.relation) {
                            tables.push(name);
                        }
                    }
                }
            },
            _ => {}
        }
        tables
    }

    /// Extract table names from a Query (SELECT ... FROM ...).
    fn extract_tables_from_query(query: &Query, tables: &mut Vec<String>) {
        if let SetExpr::Select(select) = query.body.as_ref() {
            for twj in &select.from {
                Self::extract_tables_from_join(twj, tables);
            }
        }
    }

    /// Extract table names from a table-with-joins clause.
    fn extract_tables_from_join(twj: &TableWithJoins, tables: &mut Vec<String>) {
        if let Some(name) = Self::table_factor_name(&twj.relation) {
            tables.push(name);
        }
        for join in &twj.joins {
            if let Some(name) = Self::table_factor_name(&join.relation) {
                tables.push(name);
            }
        }
    }

    /// Get the table name from a TableFactor, if it's a simple table reference.
    fn table_factor_name(factor: &TableFactor) -> Option<String> {
        match factor {
            TableFactor::Table { name, .. } => Some(name.to_string().to_lowercase()),
            _ => None,
        }
    }

    /// Extract all column references from a statement.
    fn extract_column_refs(statement: &Statement) -> Vec<(Option<String>, String)> {
        let mut cols = Vec::new();
        if let Statement::Query(query) = statement
            && let SetExpr::Select(select) = query.body.as_ref()
        {
            for item in &select.projection {
                match item {
                    SelectItem::UnnamedExpr(expr) | SelectItem::ExprWithAlias { expr, .. } => {
                        Self::extract_cols_from_expr(expr, &mut cols);
                    }
                    _ => {}
                }
            }
            if let Some(ref selection) = select.selection {
                Self::extract_cols_from_expr(selection, &mut cols);
            }
        }
        cols
    }

    /// Recursively extract column references from an expression.
    fn extract_cols_from_expr(expr: &Expr, cols: &mut Vec<(Option<String>, String)>) {
        match expr {
            Expr::Identifier(ident) => {
                cols.push((None, ident.value.to_lowercase()));
            }
            Expr::CompoundIdentifier(parts) => {
                if parts.len() == 2 {
                    cols.push((
                        Some(parts[0].value.to_lowercase()),
                        parts[1].value.to_lowercase(),
                    ));
                }
            }
            Expr::BinaryOp { left, right, .. } => {
                Self::extract_cols_from_expr(left, cols);
                Self::extract_cols_from_expr(right, cols);
            }
            Expr::IsNull(inner) | Expr::IsNotNull(inner) => {
                Self::extract_cols_from_expr(inner, cols);
            }
            Expr::Nested(inner) => {
                Self::extract_cols_from_expr(inner, cols);
            }
            Expr::Function(func) => {
                if let sqlparser::ast::FunctionArguments::List(arg_list) = &func.args {
                    for arg in &arg_list.args {
                        if let sqlparser::ast::FunctionArg::Unnamed(
                            sqlparser::ast::FunctionArgExpr::Expr(e),
                        ) = arg
                        {
                            Self::extract_cols_from_expr(e, cols);
                        }
                    }
                }
            }
            _ => {}
        }
    }

    /// Check if a binary operation has type-compatible operands.
    /// Returns issues where types clearly mismatch.
    fn check_binary_op_types(
        op: &BinaryOperator,
        left: &Expr,
        right: &Expr,
        schema: &Schema,
        tables_in_query: &[String],
    ) -> Vec<TypeIssue> {
        let mut issues = Vec::new();

        // Get types of left and right if we can resolve them
        let left_type = Self::infer_expr_type(left, schema, tables_in_query);
        let right_type = Self::infer_expr_type(right, schema, tables_in_query);

        if let (Some(lt), Some(rt)) = (&left_type, &right_type) {
            let lt_cat = type_category(lt);
            let rt_cat = type_category(rt);

            // Arithmetic operators require numeric types
            match op {
                BinaryOperator::Plus
                | BinaryOperator::Minus
                | BinaryOperator::Multiply
                | BinaryOperator::Divide
                | BinaryOperator::Modulo => {
                    if lt_cat != TypeCategory::Numeric || rt_cat != TypeCategory::Numeric {
                        issues.push(TypeIssue {
                            message: format!(
                                "Arithmetic operator {:?} used with non-numeric types: {} {:?} {}",
                                op, lt, op, rt
                            ),
                        });
                    }
                }
                // Comparison operators: both sides should be same category
                BinaryOperator::Eq
                | BinaryOperator::NotEq
                | BinaryOperator::Lt
                | BinaryOperator::LtEq
                | BinaryOperator::Gt
                | BinaryOperator::GtEq => {
                    if lt_cat != rt_cat
                        && lt_cat != TypeCategory::Unknown
                        && rt_cat != TypeCategory::Unknown
                    {
                        issues.push(TypeIssue {
                            message: format!(
                                "Comparing incompatible types: {} ({:?}) vs {} ({:?})",
                                lt, lt_cat, rt, rt_cat
                            ),
                        });
                    }
                }
                _ => {}
            }
        }

        issues
    }

    /// Attempt to infer the SQL type of an expression given the schema.
    fn infer_expr_type(expr: &Expr, schema: &Schema, tables_in_query: &[String]) -> Option<String> {
        match expr {
            Expr::Identifier(ident) => {
                let col_name = ident.value.to_lowercase();
                // Search all tables in the query for this column
                for table_name in tables_in_query {
                    if let Some(table) = schema.tables.iter().find(|t| t.name == *table_name)
                        && let Some(col) = table.columns.iter().find(|c| c.name == col_name)
                    {
                        return Some(col.col_type.clone());
                    }
                }
                None
            }
            Expr::CompoundIdentifier(parts) if parts.len() == 2 => {
                let table_name = parts[0].value.to_lowercase();
                let col_name = parts[1].value.to_lowercase();
                if let Some(table) = schema.tables.iter().find(|t| t.name == table_name)
                    && let Some(col) = table.columns.iter().find(|c| c.name == col_name)
                {
                    return Some(col.col_type.clone());
                }
                None
            }
            Expr::Value(val) => match val {
                Value::Number(_, _) => Some("numeric".to_string()),
                Value::SingleQuotedString(_) | Value::DoubleQuotedString(_) => {
                    Some("text".to_string())
                }
                Value::Boolean(_) => Some("boolean".to_string()),
                Value::Null => Some("null".to_string()),
                _ => None,
            },
            _ => None,
        }
    }
}

/// Broad type categories for compatibility checking.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TypeCategory {
    Numeric,
    Text,
    Boolean,
    Temporal,
    Unknown,
}

/// Map a SQL type string to a broad category.
fn type_category(sql_type: &str) -> TypeCategory {
    let t = sql_type.to_lowercase();
    if t.contains("int")
        || t.contains("numeric")
        || t.contains("decimal")
        || t.contains("float")
        || t.contains("double")
        || t.contains("real")
        || t.contains("serial")
    {
        TypeCategory::Numeric
    } else if t.contains("char")
        || t.contains("text")
        || t.contains("varchar")
        || t.contains("string")
    {
        TypeCategory::Text
    } else if t.contains("bool") {
        TypeCategory::Boolean
    } else if t.contains("date")
        || t.contains("time")
        || t.contains("timestamp")
        || t.contains("interval")
    {
        TypeCategory::Temporal
    } else {
        TypeCategory::Unknown
    }
}

impl QueryLanguagePlugin for SqlPlugin {
    fn name(&self) -> &str {
        &self.dialect_name
    }

    /// Level 1: Parse-time safety — is the SQL syntactically valid?
    fn parse_check(&self, query: &str) -> Result<()> {
        let _statements = self.parse(query)?;
        Ok(())
    }

    /// Level 2: Schema-binding safety — do all referenced tables and columns exist?
    fn schema_check(&self, query: &str, schema: &Schema) -> Result<Vec<SchemaIssue>> {
        let statements = self.parse(query)?;
        let mut issues = Vec::new();

        for stmt in &statements {
            // Check table references
            let table_refs = Self::extract_table_refs(stmt);
            for table_name in &table_refs {
                if !schema.tables.iter().any(|t| t.name == *table_name) {
                    issues.push(SchemaIssue {
                        message: format!("Table '{}' not found in schema", table_name),
                    });
                }
            }

            // Check column references
            let col_refs = Self::extract_column_refs(stmt);
            for (table_qualifier, col_name) in &col_refs {
                let tables_to_check: Vec<&str> = if let Some(tq) = table_qualifier {
                    vec![tq.as_str()]
                } else {
                    table_refs.iter().map(|s| s.as_str()).collect()
                };

                let found = tables_to_check.iter().any(|tn| {
                    schema
                        .tables
                        .iter()
                        .find(|t| t.name == *tn)
                        .map(|t| t.columns.iter().any(|c| c.name == *col_name))
                        .unwrap_or(false)
                });

                // Only flag if we have tables in the schema to check against
                // (don't flag columns if the table itself wasn't found — that's a Level 2 table issue)
                if !found
                    && tables_to_check
                        .iter()
                        .any(|tn| schema.tables.iter().any(|t| t.name == *tn))
                {
                    let qualified = if let Some(tq) = table_qualifier {
                        format!("{}.{}", tq, col_name)
                    } else {
                        col_name.clone()
                    };
                    issues.push(SchemaIssue {
                        message: format!("Column '{}' not found in schema", qualified),
                    });
                }
            }
        }

        Ok(issues)
    }

    /// Level 3: Type-compatible operations — no comparing strings to integers, etc.
    fn type_check(&self, query: &str, schema: &Schema) -> Result<Vec<TypeIssue>> {
        let statements = self.parse(query)?;
        let mut issues = Vec::new();

        for stmt in &statements {
            let table_refs = Self::extract_table_refs(stmt);

            // Check WHERE clause binary operations for type compatibility
            if let Statement::Query(query) = stmt
                && let SetExpr::Select(select) = query.body.as_ref()
                && let Some(ref selection) = select.selection
            {
                Self::check_expr_types(selection, schema, &table_refs, &mut issues);
            }
        }

        Ok(issues)
    }

    /// Level 4: Null safety — are nullable columns handled?
    fn null_check(&self, query: &str, schema: &Schema) -> Result<Vec<NullIssue>> {
        let statements = self.parse(query)?;
        let mut issues = Vec::new();

        for stmt in &statements {
            let table_refs = Self::extract_table_refs(stmt);

            // Check if SELECT includes nullable columns without COALESCE or IS NULL handling
            if let Statement::Query(q) = stmt
                && let SetExpr::Select(select) = q.body.as_ref()
            {
                for item in &select.projection {
                    match item {
                        SelectItem::UnnamedExpr(Expr::Identifier(ident))
                        | SelectItem::ExprWithAlias {
                            expr: Expr::Identifier(ident),
                            ..
                        } => {
                            let col_name = ident.value.to_lowercase();
                            for table_name in &table_refs {
                                if let Some(table) =
                                    schema.tables.iter().find(|t| t.name == *table_name)
                                    && let Some(col) =
                                        table.columns.iter().find(|c| c.name == col_name)
                                    && col.nullable
                                {
                                    issues.push(NullIssue {
                                                    message: format!(
                                                        "Nullable column '{}' selected without COALESCE or null handling",
                                                        col_name
                                                    ),
                                                    column: col_name.clone(),
                                                });
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        Ok(issues)
    }
}

impl SqlPlugin {
    /// Recursively check expressions for type issues.
    fn check_expr_types(
        expr: &Expr,
        schema: &Schema,
        tables: &[String],
        issues: &mut Vec<TypeIssue>,
    ) {
        match expr {
            Expr::BinaryOp { left, op, right } => {
                let new_issues = Self::check_binary_op_types(op, left, right, schema, tables);
                issues.extend(new_issues);
                Self::check_expr_types(left, schema, tables, issues);
                Self::check_expr_types(right, schema, tables, issues);
            }
            Expr::Nested(inner) => Self::check_expr_types(inner, schema, tables, issues),
            _ => {}
        }
    }
}
