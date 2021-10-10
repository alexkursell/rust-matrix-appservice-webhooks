use anyhow::Result;
use sqlx::{sqlite::SqliteConnectOptions, Executor, SqlitePool};

#[derive(Debug)]
pub struct Store(SqlitePool);

#[derive(Debug, PartialEq, sqlx::FromRow)]
#[sqlx(rename_all = "camelCase")]
pub struct Webhook {
  pub id: String,
  pub room_id: String,
  pub user_id: String,
  pub label: Option<String>,
}

impl Store {
  pub async fn connect(path: &str) -> Result<Self> {
    let opts = SqliteConnectOptions::new()
      .filename(path)
      .create_if_missing(true);
    let conn = SqlitePool::connect_with(opts).await?;
    conn
      .execute(sqlx::query(
        r#"CREATE TABLE IF NOT EXISTS "webhooks" 
    (
      "id" VARCHAR  PRIMARY KEY NOT NULL, 
      "roomId" VARCHAR  NOT NULL, 
      "userId" VARCHAR  NOT NULL, 
      "label" VARCHAR
    );"#,
      ))
      .await?;

    Ok(Self(conn))
  }

  pub async fn create_webhook(&self, room_id: &str, user_id: &str) -> Result<Webhook> {
    let id = randid::randid_str(32);
    let hook = Webhook {
      id,
      room_id: room_id.to_string(),
      user_id: user_id.to_string(),
      label: None,
    };

    sqlx::query("INSERT INTO webhooks ( id, roomId, userId, label ) VALUES ( ?1, ?2, ?3, null );")
      .bind(&hook.id)
      .bind(&hook.room_id)
      .bind(&hook.user_id)
      .execute(&mut (self.0.acquire().await?))
      .await?;

    Ok(hook)
  }

  pub async fn get_webhook_by_id(&self, id: &str) -> Result<Option<Webhook>> {
    let possible: Option<Webhook> =
      sqlx::query_as::<_, Webhook>("SELECT * FROM webhooks WHERE id = ?")
        .bind(id)
        .fetch_optional(&mut (self.0.acquire().await?))
        .await?;

    Ok(possible)
  }
}

mod tests {

  #[tokio::test]
  async fn test_basic() {
    let s = super::Store::connect("sqlite::memory:").await.unwrap();

    let h1 = s.create_webhook("room1", "userblah").await.unwrap();
    let id = h1.id.clone();

    assert_eq!(Some(h1), s.get_webhook_by_id(&id).await.unwrap());
  }
}
