//! Contains struct used for handling connection and queries
//! to PostgreSQL/SQLite database used by ACSIM

use sqlx::{any::AnyPoolOptions, AnyPool, Row};

/// Deserialized DB row containing a message (thread)
#[derive(Debug, sqlx::FromRow)]
pub struct MessageRow {
    pub msgid: i64,
    pub board: String,
    pub time: i64,
    pub author: String,
    pub msg: String,
    pub image: String,
    pub latest_submsg: i64,
}

/// Deserialized DB row containing a submessage (post)
#[derive(Debug, sqlx::FromRow)]
pub struct SubmessageRow {
    pub parent_msg: i64,
    pub submsg_id: i64,
    pub board: String,
    pub time: i64,
    pub author: String,
    pub submsg: String,
    pub image: String,
}

/// Deserialized data about flagged messages/submessages
#[derive(Debug, sqlx::FromRow)]
pub struct FlaggedRow {
    pub entry_id: i64,
    pub msg_type: String,
    pub msgid: i64,
    pub submsg_index: Option<i64>,
}

/// Wrapper for the DB client
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

    pub async fn count_board_submessages(&self, board: &str) -> Result<i64, sqlx::Error> {
        let count_struct = sqlx::query("SELECT COUNT(submsg_id) FROM submessages WHERE board=$1")
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

    pub async fn get_posting_rate(
        &self,
        board: &str,
        time_period: i64,
    ) -> Result<i64, sqlx::Error> {
        let count_struct = sqlx::query("SELECT COUNT(*) FROM (SELECT board, time FROM messages UNION SELECT board, time FROM submessages) WHERE board=$1 AND time > $2")
            .bind(board)
            // now we select all messages sent later than "current time" - `time_period` seconds ago
            .bind(crate::html_proc::since_epoch() - time_period)
            .fetch_one(&self.db_pool)
            .await;
        Ok(count_struct.unwrap().try_get(0).unwrap())
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
                "SELECT * FROM (SELECT * FROM messages WHERE board=$1 ORDER BY latest_submsg DESC LIMIT $4 OFFSET $3) AS limited WHERE limited.msg LIKE '%' || $2 || '%'")
                .bind(board.to_string()).bind(substring.to_string()).bind(page).bind(limit)
            .fetch_all(&self.db_pool).await
    }

    pub async fn get_last_message(&self, board: &str) -> Result<MessageRow, sqlx::Error> {
        sqlx::query_as::<_, MessageRow>(
            "SELECT * FROM messages WHERE board=$1 ORDER BY latest_submsg DESC LIMIT 1",
        )
        .bind(board)
        .fetch_one(&self.db_pool)
        .await
    }

    pub async fn get_last_submessage(
        &self,
        parent_msgid: &i64,
    ) -> Result<SubmessageRow, sqlx::Error> {
        sqlx::query_as::<_, SubmessageRow>(
            "SELECT * FROM submessages WHERE parent_msg=$1 ORDER BY time DESC LIMIT 1",
        )
        .bind(parent_msgid)
        .fetch_one(&self.db_pool)
        .await
    }

    pub async fn get_single_message(&self, msgid: i64) -> Result<MessageRow, sqlx::Error> {
        sqlx::query_as::<_, MessageRow>("SELECT * FROM messages WHERE msgid=$1")
            .bind(msgid)
            .fetch_one(&self.db_pool)
            .await
    }

    pub async fn get_single_submessage(
        &self,
        parent_id: i64,
        submsgid: i64,
    ) -> Result<SubmessageRow, sqlx::Error> {
        sqlx::query_as::<_, SubmessageRow>(
            "SELECT * FROM submessages WHERE parent_msg=$1 AND submsg_id=$2",
        )
        .bind(parent_id)
        .bind(submsgid)
        .fetch_one(&self.db_pool)
        .await
    }

    pub async fn get_flagged_messages(&self) -> Result<Vec<MessageRow>, sqlx::Error> {
        sqlx::query_as::<_, MessageRow>("SELECT * FROM messages WHERE msgid IN (SELECT msgid FROM flagged_messages WHERE msg_type='msg') ORDER BY time DESC")
            .fetch_all(&self.db_pool)
            .await
    }

    pub async fn get_flagged_submessages(&self) -> Result<Vec<SubmessageRow>, sqlx::Error> {
        sqlx::query_as::<_, SubmessageRow>("SELECT * FROM submessages WHERE parent_msg IN (SELECT msgid FROM flagged_messages WHERE msg_type='submsg') AND submsg_id IN (SELECT submsg_index FROM flagged_messages) ORDER BY time DESC")
            .fetch_all(&self.db_pool)
            .await
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

    pub async fn delete_msg(&self, msgid: i64) {
        DatabaseWrapper::log_query_status(
            sqlx::query("DELETE FROM messages WHERE msgid=$1")
                .bind(msgid)
                .execute(&self.db_pool)
                .await,
            "Deleting message",
        );
    }

    pub async fn delete_submsg(&self, msgid: i64, submsgid: i64) {
        DatabaseWrapper::log_query_status(
            sqlx::query("DELETE FROM submessages WHERE parent_msg=$1 AND submsg_id=$2")
                .bind(msgid)
                .bind(submsgid)
                .execute(&self.db_pool)
                .await,
            "Deleting submessage",
        );
    }

    pub async fn insert_to_messages(
        &self,
        board: &str,
        time: i64,
        author: &str,
        msg: &str,
        image: &str,
        latest_submsg: i64,
    ) {
        DatabaseWrapper::log_query_status(
            sqlx::query("INSERT INTO messages(board, time, author, msg, image, latest_submsg) VALUES ($1, $2, $3, $4, $5, $6)")
            .bind(board.to_string()).bind(time).bind(author.to_string()).bind(msg.to_string()).bind(image.to_string()).bind(latest_submsg).execute(&self.db_pool).await, "Inserting row into messages table"
        );
    }

    pub async fn insert_to_submessages(
        &self,
        parent_msg: i64,
        submsg_id: i64,
        board: &str,
        time: i64,
        author: &str,
        submsg: &str,
        image: &str,
    ) {
        DatabaseWrapper::log_query_status(
            sqlx::query("INSERT INTO submessages(parent_msg, submsg_id, board, time, author, submsg, image) VALUES ($1, $2, $3, $4, $5, $6, $7)")
            .bind(parent_msg).bind(submsg_id).bind(board.to_string()).bind(time).bind(author.to_string()).bind(submsg.to_string()).bind(image.to_string()).execute(&self.db_pool).await, "Inserting row into submessages table"
        );
    }

    pub async fn insert_to_flagged(&self, msg_type: String, msgid: i64, submsgid: Option<i64>) {
        DatabaseWrapper::log_query_status(
            sqlx::query(
                "INSERT INTO flagged_messages(msg_type, msgid, submsg_index) VALUES ($1, $2, $3)",
            )
            .bind(msg_type)
            .bind(msgid)
            .bind(submsgid.unwrap_or(0))
            .execute(&self.db_pool)
            .await,
            "Inserting row into flagged_messages table",
        );
    }
}
