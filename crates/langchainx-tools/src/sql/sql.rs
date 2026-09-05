//! Database dialects, engine interface, and schema-aware query wrapper.
use std::{collections::HashSet, error::Error, fmt};

use async_trait::async_trait;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
/// SQL dialect reported by an [`Engine`].
pub enum Dialect {
    #[serde(rename = "mysql")]
    /// MySQL syntax.
    MySQL,
    #[serde(rename = "sqlite")]
    /// SQLite syntax.
    SQLite,
    #[serde(rename = "postgresql")]
    /// PostgreSQL syntax.
    PostgreSQL,
}
impl fmt::Display for Dialect {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Dialect::MySQL => "mysql",
            Dialect::SQLite => "sqlite",
            Dialect::PostgreSQL => "postgresql",
        };
        write!(f, "{}", s)
    }
}

#[async_trait]
/// Database operations needed by [`SQLDatabase`].
pub trait Engine: Send + Sync {
    // Dialect returns the dialect(e.g. mysql, sqlite, postgre) of the database.
    /// Returns the database SQL dialect.
    fn dialect(&self) -> Dialect;
    // Query executes the query and returns the columns and results.
    /// Executes SQL and returns column names plus stringified rows.
    async fn query(&self, query: &str) -> Result<(Vec<String>, Vec<Vec<String>>), Box<dyn Error>>;
    // TableNames returns all the table names of the database.
    /// Lists tables available to the current connection.
    async fn table_names(&self) -> Result<Vec<String>, Box<dyn Error>>;
    // TableInfo returns the table information of the database.
    // Typically, it returns the CREATE TABLE statement.
    /// Describes a table, typically as a `CREATE TABLE` statement.
    async fn table_info(&self, tables: &str) -> Result<String, Box<dyn Error>>;
    // Close closes the database.
    /// Releases or closes underlying database resources.
    fn close(&self) -> Result<(), Box<dyn Error>>;
}

/// Schema-aware wrapper around a database engine.
pub struct SQLDatabase {
    /// Database engine used for all operations.
    pub engine: Box<dyn Engine>,
    /// Number of sample rows included in table descriptions.
    pub sample_rows_number: i32,
    /// Tables visible after applying the builder's ignore list.
    pub all_tables: HashSet<String>,
}

/// Configures table filtering and schema sampling for [`SQLDatabase`].
pub struct SQLDatabaseBuilder {
    engine: Box<dyn Engine>,
    sample_rows_number: i32,
    ignore_tables: HashSet<String>,
}

const DEFAULT_SAMPLE_ROWS: i32 = 3;

impl SQLDatabaseBuilder {
    /// Creates a builder around the supplied database engine.
    pub fn new<E>(engine: E) -> Self
    where
        E: Engine + 'static,
    {
        SQLDatabaseBuilder {
            engine: Box::new(engine),
            sample_rows_number: DEFAULT_SAMPLE_ROWS,
            ignore_tables: HashSet::new(),
        }
    }

    // Function to set custom number of sample rows
    /// Sets the number of rows sampled in table descriptions.
    pub fn custom_sample_rows_number(mut self, number: i32) -> Self {
        self.sample_rows_number = number;
        self
    }

    // Function to set tables to ignore
    /// Excludes the supplied tables from the database view.
    pub fn ignore_tables(mut self, ignore_tables: HashSet<String>) -> Self {
        self.ignore_tables = ignore_tables;
        self
    }

    // Function to build the SQLDatabase instance
    /// Loads table names and builds the filtered database view.
    pub async fn build(self) -> Result<SQLDatabase, Box<dyn Error>> {
        let table_names_result = self.engine.table_names().await;

        // Handle potential error from table_names call
        let table_names = match table_names_result {
            Ok(names) => names,
            Err(error) => {
                return Err(error);
            }
        };

        // Filter out ignored tables
        let all_tables: HashSet<String> = table_names
            .into_iter()
            .filter(|name| !self.ignore_tables.contains(name))
            .collect();

        Ok(SQLDatabase {
            engine: self.engine,
            sample_rows_number: self.sample_rows_number,
            all_tables,
        })
    }
}

impl SQLDatabase {
    /// Returns the underlying engine's SQL dialect.
    pub fn dialect(&self) -> Dialect {
        self.engine.dialect()
    }

    /// Returns the visible table names in unspecified order.
    pub fn table_names(&self) -> Vec<String> {
        self.all_tables.iter().cloned().collect()
    }

    // qual:allow(iosp) reason: "database I/O boundary"
    /// Returns schema descriptions and optional sample rows for selected tables.
    pub async fn table_info(&self, tables: &[String]) -> Result<String, Box<dyn Error>> {
        let mut tables: HashSet<String> = tables.iter().cloned().collect();
        if tables.is_empty() {
            tables = self.all_tables.clone();
        }
        let mut info = String::new();
        for table in tables {
            let table_info = self.engine.table_info(&table).await?;
            info.push_str(&table_info);
            info.push_str("\n\n");

            if self.sample_rows_number > 0 {
                let sample_rows = self.sample_rows(&table).await?;
                info.push_str("/*\n");
                info.push_str(&sample_rows);
                info.push_str("*/ \n\n");
            }
        }
        Ok(info)
    }

    /// Executes SQL and formats the columns and rows as tab-separated text.
    pub async fn query(&self, query: &str) -> Result<String, Box<dyn Error>> {
        log::debug!("Query: {}", query);
        let (cols, results) = self.engine.query(query).await?;
        let mut str = cols.join("\t") + "\n";
        for row in results {
            str += &row.join("\t");
            str.push('\n');
        }
        Ok(str)
    }

    /// Delegates resource cleanup to the underlying engine.
    pub fn close(&self) -> Result<(), Box<dyn Error>> {
        self.engine.close()
    }

    /// Selects and formats the configured number of rows from a table.
    pub async fn sample_rows(&self, table: &str) -> Result<String, Box<dyn Error>> {
        let query = format!("SELECT * FROM {} LIMIT {}", table, self.sample_rows_number);
        log::debug!("Sample Rows Query: {}", query);
        self.query(&query).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    struct StubEngine {
        dialect: Dialect,
    }

    #[async_trait]
    impl Engine for StubEngine {
        fn dialect(&self) -> Dialect {
            match self.dialect {
                Dialect::MySQL => Dialect::MySQL,
                Dialect::SQLite => Dialect::SQLite,
                Dialect::PostgreSQL => Dialect::PostgreSQL,
            }
        }

        async fn query(
            &self,
            _query: &str,
        ) -> Result<(Vec<String>, Vec<Vec<String>>), Box<dyn Error>> {
            Ok((vec![], vec![]))
        }

        async fn table_names(&self) -> Result<Vec<String>, Box<dyn Error>> {
            Ok(vec![])
        }

        async fn table_info(&self, _table: &str) -> Result<String, Box<dyn Error>> {
            Ok(String::new())
        }

        fn close(&self) -> Result<(), Box<dyn Error>> {
            Ok(())
        }
    }

    fn make_db(dialect: Dialect) -> SQLDatabase {
        SQLDatabase {
            engine: Box::new(StubEngine { dialect }),
            sample_rows_number: 0,
            all_tables: HashSet::new(),
        }
    }

    #[test]
    fn dialect_returns_engine_dialect() {
        let db = make_db(Dialect::SQLite);
        assert!(matches!(db.dialect(), Dialect::SQLite));
    }

    #[test]
    fn close_returns_ok_on_stub_engine() {
        let db = make_db(Dialect::PostgreSQL);
        assert!(db.close().is_ok());
    }
}
