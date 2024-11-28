use sqlx::postgres::PgPool;

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct User {
    id: i32,
    username: String,
    email: String,
    password_hash: String,
    created_at: DateTime<Utc>,
}

async fn create_user(
    pool: &PgPool,
    username: &str,
    email: &str,
    password_hash: &str,
) -> anyhow::Result<i32> {
    let record = sqlx::query!(
        r#"
        INSERT INTO users (username, email, password_hash)
        VALUES ($1, $2, $3)
        RETURNING id
        "#,
        username,
        email,
        password_hash
    )
    .fetch_one(pool)
    .await?;

    Ok(record.id)
}

async fn get_user_by_id(pool: &PgPool, user_id: i32) -> anyhow::Result<Option<User>> {
    let user = sqlx::query_as!(
        User,
        r#"
        SELECT id, username, email, password_hash, created_at
        FROM users
        WHERE id = $1
        "#,
        user_id
    )
    .fetch_optional(pool)
    .await?;

    Ok(user)
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    let database_url = std::env::var("DATABASE_URL")?;
    let pool = PgPool::connect(&database_url).await?;

    let username = "alice";
    let email = "alice@example.com";
    let password_hash = "password123";
    create_user(&pool, username, email, password_hash).await?;

    let user = get_user_by_id(&pool, 1).await?;
    println!("{:?}", user);

    Ok(())
}
