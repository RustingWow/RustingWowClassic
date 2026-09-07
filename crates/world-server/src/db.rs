use sqlx::postgres::PgPoolOptions;
use sqlx::Executor;

pub async fn migrate(database_url: &str) -> anyhow::Result<()> {
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
    sqlx::migrate!("./migrations").run(&pool).await?;
    pool.close().await;
    Ok(())
}
