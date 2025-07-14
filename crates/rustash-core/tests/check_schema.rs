use diesel::prelude::*;
use diesel::sql_query;
use diesel::sql_types::*;
use rustash_core::database::create_test_pool;

// Struct to represent the result of a COUNT(*) query
#[derive(QueryableByName)]
struct TableCount {
    #[diesel(sql_type = Integer)]
    count: i32,
}

// Struct to represent a column in a SQLite table
#[derive(QueryableByName, Debug)]
struct ColumnInfo {
    #[diesel(sql_type = Integer)]
    cid: i32,
    #[diesel(sql_type = Text)]
    name: String,
    #[diesel(sql_type = Text)]
    type_: String,
    #[diesel(sql_type = Integer)]
    notnull: i32,
    #[diesel(sql_type = Nullable<Text>)]
    dflt_value: Option<String>,
    #[diesel(sql_type = Integer)]
    pk: i32,
}

#[tokio::test]
async fn check_schema() -> Result<(), Box<dyn std::error::Error>> {
    // Create a test pool
    let pool = create_test_pool().await?;
    let mut conn = pool.get_connection().await?;

    // Check if the snippets table exists
    let table_count: TableCount = diesel::sql_query(
        "SELECT COUNT(*) as count FROM sqlite_master WHERE type='table' AND name='snippets'",
    )
    .get_result(&mut *conn)
    .await?;

    assert!(table_count.count > 0, "'snippets' table is missing");

    // Validate the schema of the snippets table
    let schema: Vec<ColumnInfo> = diesel::sql_query("PRAGMA table_info(snippets)")
        .load(&mut *conn)
        .await?;

    let column_names: std::collections::HashSet<String> =
        schema.iter().map(|c| c.name.clone()).collect();

    let expected_columns = [
        "uuid",
        "title",
        "content",
        "tags",
        "embedding",
        "created_at",
        "updated_at",
    ];

    for col in expected_columns {
        assert!(
            column_names.contains(col),
            "column '{}' missing from 'snippets' table",
            col
        );
    }

    // Check if the snippets_old table exists
    let old_table_count: TableCount = diesel::sql_query(
        "SELECT COUNT(*) as count FROM sqlite_master WHERE type='table' AND name='snippets_old'",
    )
    .get_result(&mut *conn)
    .await?;

    let old_table_exists = old_table_count.count > 0;
    println!("\nsnippets_old table exists: {}", old_table_exists);

    // Get the schema of the snippets_old table if it exists
    if old_table_exists {
        let schema: Vec<ColumnInfo> = diesel::sql_query("PRAGMA table_info(snippets_old)")
            .load(&mut *conn)
            .await?;

        println!("\nSchema for 'snippets_old' table:");
        println!(
            "{: <5} {: <15} {: <15} {: <5} {: <10} {: <5}",
            "cid", "name", "type", "notnull", "dflt_value", "pk"
        );
        for col in schema {
            println!(
                "{: <5} {: <15} {: <15} {: <5} {: <10} {: <5}",
                col.cid,
                col.name,
                col.type_,
                col.notnull,
                col.dflt_value.unwrap_or_default(),
                col.pk
            );
        }
    }

    // Check if the __diesel_schema_migrations table exists
    let migrations_table_count: TableCount = diesel::sql_query(
        "SELECT COUNT(*) as count FROM sqlite_master WHERE type='table' AND name='__diesel_schema_migrations'"
    )
    .get_result(&mut *conn)
    .await?;

    assert!(
        migrations_table_count.count > 0,
        "'__diesel_schema_migrations' table is missing"
    );

    // Get the applied migrations
    {
        #[derive(QueryableByName, Debug)]
        struct Migration {
            #[diesel(sql_type = Text)]
            version: String,
            #[diesel(sql_type = Integer)]
            run_on: i32, // SQLite doesn't have a native boolean type
        }

        let migrations: Vec<Migration> =
            sql_query("SELECT version, run_on FROM __diesel_schema_migrations ORDER BY version")
                .load(&mut *conn)
                .await?;

        println!("\nApplied migrations:");
        for migration in migrations {
            println!("- {} (run_on: {})", migration.version, migration.run_on);
        }
    }

    Ok(())
}
