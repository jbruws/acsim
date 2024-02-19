//! Struct used for handling connection and queries
//! to PostgreSQL database used by ACSIM.

use sqlx::{any::AnyPoolOptions, AnyPool, Row};

/// Struct used to deserialize messages from DB rows
#[derive(Debug, sqlx::FromRow)]
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
#[derive(Debug, sqlx::FromRow)]
pub struct SubmessageRow {
    pub parent_msg: i64,
    pub time: i64,
    pub author: String,
    pub submsg: String,
    pub image: String,
}

/// Struct used to deserialize flagged messages and submessages 
#[derive(Debug, sqlx::FromRow)]
pub struct FlaggedRow {
    pub msg_type: String,
    pub msgid: i64,
    pub submsg_index: i64,
}

/// Wrapper for PostgreSQL DB client
pub struct DatabaseWrapper {
    db_pool: AnyPool,
}

impl DatabaseWrapper {
    pub async fn new() -> Result<DatabaseWrapper, sqlx::Error> {
        // loading database drivers
        sqlx::any::install_default_drivers();

        let url = match std::env::var("DATABASE_URL") {
            Ok(u) => u,
            Err(_) => panic!("DATABASE_URL variable is not set (check .env file)"),
        };

        // connecting to the database
        let pool = AnyPoolOptions::new().connect(&url).await?;

        Ok(DatabaseWrapper { db_pool: pool })
    }

    fn log_query_status<T: core::fmt::Debug>(status: Result<T, sqlx::Error>, operation: &str) {
        match status {
            Ok(v) => log::debug!("{} success: {:?}", operation, v),
            Err(e) => log::error!("{} failure: {:?}", operation, e),
        };
    }

    pub async fn count_messages(&self, board: &str) -> Result<i64, sqlx::Error> {
        let count_struct = sqlx::query("SELECT COUNT(msgid) FROM messages WHERE board=$1")
            .bind(String::from(board))
            .fetch_one(&self.db_pool)
            .await;
        Ok(count_struct.unwrap().try_get(0).unwrap())
    }

    pub async fn count_submessages(&self, msgid: i64) -> Result<i64, sqlx::Error> {
        let count_struct = sqlx::query("SELECT COUNT(*) FROM submessages WHERE parent_msg=$1")
            .bind(msgid)
            .fetch_one(&self.db_pool)
            .await;
        Ok(count_struct.unwrap().try_get(0).unwrap())
    }

    pub async fn delete_least_active(&self, board: &str) {
        DatabaseWrapper::log_query_status(
            sqlx::query("DELETE FROM messages WHERE latest_submsg = (SELECT MIN(latest_submsg) FROM messages WHERE board=$1)")
            .bind(String::from(board))
            .execute(&self.db_pool).await,
            "Deleting least active message"
        );
    }

    pub async fn get_submessages(&self, msgid: i64) -> Result<Vec<SubmessageRow>, sqlx::Error> {
        sqlx::query_as::<_, SubmessageRow>("SELECT * FROM submessages WHERE parent_msg=$1")
            .bind(msgid)
            .fetch_all(&self.db_pool)
            .await
    }

    pub async fn get_messages(
        &self,
        board: &str,
        page: i64,
        limit: i64,
    ) -> Result<Vec<MessageRow>, sqlx::Error> {
        sqlx::query_as::<_, MessageRow>(
            "SELECT * FROM messages WHERE board=$1 ORDER BY latest_submsg DESC LIMIT $3 OFFSET $2",
        )
        .bind(board.to_string())
        .bind((page - 1) * limit)
        .bind(limit)
        .fetch_all(&self.db_pool)
        .await
    }

    pub async fn search_messages(
        &self,
        board: &str,
        page: i64,
        limit: i64,
        substring: &str,
    ) -> Result<Vec<MessageRow>, sqlx::Error> {
        sqlx::query_as::<_, MessageRow>(
                "SELECT * FROM (SELECT * FROM messages WHERE board=$1 ORDER BY latest_submsg DESC OFFSET $3 LIMIT $4) AS limited WHERE limited.msg LIKE '%' || $2 || '%'")
                .bind(board.to_string()).bind(substring.to_string()).bind(page).bind(limit)
            .fetch_all(&self.db_pool).await
    }

    pub async fn get_last_message(&self, board: &str) -> Result<MessageRow, sqlx::Error> {
        match sqlx::query_as::<_, MessageRow>(
            "SELECT * FROM messages WHERE board=$1 ORDER BY latest_submsg DESC LIMIT 1",
        )
        .bind(board)
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

    pub async fn get_last_submessage(
        // duplicate code. bruh...
        &self,
        parent_msgid: &i64,
    ) -> Result<SubmessageRow, sqlx::Error> {
        match sqlx::query_as::<_, SubmessageRow>(
            "SELECT * FROM submessages WHERE parent_msg=$1 ORDER BY time DESC LIMIT 1",
        )
        .bind(parent_msgid)
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

    pub async fn get_single_message(&self, msgid: i64) -> Result<MessageRow, sqlx::Error> {
        match sqlx::query_as::<_, MessageRow>("SELECT * FROM messages WHERE msgid=$1")
            .bind(msgid)
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
            sqlx::query("UPDATE messages SET latest_submsg = $1 WHERE msgid = $2")
                .bind(msgid.to_string())
                .bind(since_epoch.to_string())
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
        DatabaseWrapper::log_query_status(
            sqlx::query("INSERT INTO messages(time, author, msg, image, latest_submsg, board) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(time).bind(author.to_string()).bind(msg.to_string()).bind(image.to_string()).bind(latest_submsg).bind(board.to_string()).execute(&self.db_pool).await, "Inserting row into messages table"
        );
    }

    pub async fn insert_to_submessages(
        &self,
        parent_msg: i64,
        time: i64,
        author: &str,
        submsg: &str,
        image: &str,
    ) {
        DatabaseWrapper::log_query_status(
            sqlx::query("INSERT INTO submessages(parent_msg, time, author, submsg, image) VALUES ($1, $2, $3, $4, $5)")
            .bind(parent_msg).bind(time).bind(author.to_string()).bind(submsg.to_string()).bind(image.to_string()).execute(&self.db_pool).await, "Inserting row into submessages table"
        );
    }
}
