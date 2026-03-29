use sqlx::{PgPool, postgres::PgPoolOptions};
use std::env;

pub async fn init_db() -> PgPool {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL environment variable not set");
    
    PgPoolOptions::new()
        .max_connections(10)
        .min_connections(2)
        .acquire_timeout(std::time::Duration::from_secs(5))
        .connect(&database_url)
        .await
        .expect("Failed to create database connection pool")
}
