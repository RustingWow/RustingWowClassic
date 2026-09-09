use sqlx::postgres::PgPoolOptions;
use sqlx::{Executor, PgPool};

pub async fn connect_world(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .after_connect(|conn, _meta| {
            Box::pin(async move {
                conn.execute("CREATE SCHEMA IF NOT EXISTS world").await?;
                conn.execute("SET search_path TO world").await?;
                Ok(())
            })
        })
        .connect(database_url)
        .await?;
    // `migrate!` embeds SQL at compile time; build.rs watches ./migrations for new files.
    sqlx::migrate!("./migrations").run(&pool).await?;
    Ok(pool)
}
