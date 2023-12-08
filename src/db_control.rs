//! Struct used for handling connection and queries
//! to PostgreSQL database used by ACSIM.

//use tokio_postgres::error::Error;
//use tokio_postgres::row::Row;

use sqlx::{postgres::PgPoolOptions, PgPool, Pool, Row};

/// Struct used to deserialize messages from DB rows
#[derive(Debug)]
pub struct MessageRow {
    pub msgid: i64,
    pub time: i64,
    pub author: String,
    pub msg: String,
    pub image: String,
    pub latest_submsg: i64,
    pub board: String,
}

/// Struct used to deserialize submessages from DB rows
#[derive(Debug)]
pub struct SubmessageRow {
    pub parent_msg: i64,
    pub time: i64,
    pub author: String,
    pub submsg: String,
    pub image: String,
}

/// Wrapper for PostgreSQL DB client
pub struct DatabaseWrapper {
    // There used to be a `tokio_postgres::Client`
    // but I moved to sqlx
    db_pool: PgPool,
}

impl DatabaseWrapper {
    pub async fn new() -> Result<DatabaseWrapper, sqlx::Error> {
        //let db_host = std::env::var("DB_HOST").expect("Must specify DB host in .env");
        //let db_user = std::env::var("DB_USER").expect("Must specify DB user in .env");
        //let db_password = match std::env::var("DB_PASSWORD") {
            //Ok(p) => format!(":{}", p),
            //Err(_) => "".to_string(),
        //};

        // You must set DATABASE_URL at compile time, i. e. through `.env`. Setting it with
        // std::env does not work. What a shame

        // connecting to the database
        let pool = PgPoolOptions::new().connect(&std::env::var("DATABASE_URL").unwrap()).await?;

        Ok(DatabaseWrapper { db_pool: pool })
    }

    fn log_query_status<T: core::fmt::Debug>(status: Result<T, sqlx::Error>, operation: &str) {
        match status {
            Ok(v) => log::debug!("{} success: {:?}", operation, v),
            Err(e) => log::error!("{} failure: {:?}", operation, e),
        };
    }

    pub async fn count_messages(&self, board: &str) -> Result<i64, sqlx::Error> {
        let count_struct = sqlx::query!("SELECT COUNT(msgid) FROM messages WHERE board=$1", board,)
            .fetch_one(&self.db_pool)
            .await;
        Ok(count_struct.unwrap().count.unwrap())
    }

    pub async fn count_submessages(&self, msgid: i64) -> Result<i64, sqlx::Error> {
        let count_struct = sqlx::query!(
            "SELECT COUNT(*) FROM submessages WHERE parent_msg=$1",
            msgid,
        )
        .fetch_one(&self.db_pool)
        .await;
        Ok(count_struct.unwrap().count.unwrap())
    }

    pub async fn delete_least_active(&self, board: &str) {
        DatabaseWrapper::log_query_status(sqlx::query!("DELETE FROM messages WHERE latest_submsg = (SELECT MIN(latest_submsg) FROM messages WHERE board=$1)", board).execute(&self.db_pool).await, "Deleting least active message");
    }

    pub async fn get_submessages(&self, msgid: i64) -> Result<Vec<SubmessageRow>, sqlx::Error> {
        sqlx::query_as!(
            SubmessageRow,
            "SELECT * FROM submessages WHERE parent_msg=$1",
            msgid
        )
        .fetch_all(&self.db_pool)
        .await
    }

    pub async fn get_messages(
        &self,
        board: &str,
        page: i64,
        limit: i64,
    ) -> Result<Vec<MessageRow>, sqlx::Error> {
        sqlx::query_as!(
                MessageRow,
                "SELECT * FROM messages WHERE board=$1 AND (msgid BETWEEN $2 AND $3) ORDER BY latest_submsg DESC",
                board, (&limit * (&page - 1)), (&limit * &page),
            )
            .fetch_all(&self.db_pool).await
    }

    pub async fn search_messages(
        &self,
        board: &str,
        page: i64,
        limit: i64,
        substring: &str,
    ) -> Result<Vec<MessageRow>, sqlx::Error> {
        sqlx::query_as!(
                MessageRow,
                "SELECT * FROM (SELECT * FROM messages WHERE board=$1 ORDER BY latest_submsg DESC OFFSET $3 LIMIT $4) AS limited WHERE limited.msg LIKE '%' || $2 || '%'",
                board, substring, page, limit,
            )
            .fetch_all(&self.db_pool).await
    }

    pub async fn get_single_message(&self, msgid: i64) -> Result<MessageRow, sqlx::Error> {
        match sqlx::query_as!(MessageRow, "SELECT * FROM messages WHERE msgid=$1", msgid)
            .fetch_optional(&self.db_pool)
            .await
        {
            Ok(val) => match val {
                Some(r) => Ok(r),
                None => Err(sqlx::Error::RowNotFound), //wha??
            },
            Err(e) => Err(e),
        }
    }

    pub async fn update_message_activity(&self, msgid: i64, since_epoch: i64) {
        DatabaseWrapper::log_query_status(
            sqlx::query!(
                "UPDATE messages SET latest_submsg = $1 WHERE msgid = $2",
                msgid,
                since_epoch,
            )
            .execute(&self.db_pool)
            .await,
            "Updating message activity timer",
        );
    }

    pub async fn insert_to_messages(
        &self,
        time: i64,
        author: &str,
        msg: &str,
        image: &str,
        latest_submsg: i64,
        board: &str,
    ) {
        DatabaseWrapper::log_query_status(sqlx::query!("INSERT INTO messages(time, author, msg, image, latest_submsg, board) VALUES ($1, $2, $3, $4, $5, $6)", time, author, msg, image, latest_submsg, board).execute(&self.db_pool).await, "Inserting row into messages table");
    }

    pub async fn insert_to_submessages(
        &self,
        parent_msg: i64,
        time: i64,
        author: &str,
        submsg: &str,
        image: &str,
    ) {
        DatabaseWrapper::log_query_status(sqlx::query!("INSERT INTO submessages(parent_msg, time, author, submsg, image) VALUES ($1, $2, $3, $4, $5)", parent_msg, time, author, submsg, image).execute(&self.db_pool).await, "Inserting row into submessages table");
    }
}
