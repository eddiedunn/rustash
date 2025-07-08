use rustash_core::database::create_test_pool;
use diesel::prelude::*;
use diesel::sql_query;

#[test]
fn check_applied_migrations() -> Result<(), Box<dyn std::error::Error>> {
    // Create a test pool
    let pool = create_test_pool()?;
    let mut conn = pool.get()?;

    // Query the migrations table
    #[derive(QueryableByName, Debug)]
    struct Migration {
        #[diesel(sql_type = diesel::sql_types::Text)]
        version: String,
        #[diesel(sql_type = diesel::sql_types::Bool)]
        run_on: bool,
    }

    let migrations: Vec<Migration> = sql_query(
        "SELECT version, run_on FROM __diesel_schema_migrations ORDER BY version"
    ).load(&mut *conn)?;

    println!("Applied migrations:");
    for migration in migrations {
        println!("- {} (run_on: {})", migration.version, migration.run_on);
    }

    Ok(())
}
