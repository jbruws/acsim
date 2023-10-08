//! Struct used for handling connection and queries
//! to PostgreSQL database used by ACSIM.

use tokio_postgres::error::Error;
use tokio_postgres::row::Row;

/// Wrapper for PostgreSQL DB client
pub struct DatabaseWrapper {
    client: tokio_postgres::Client,
}

impl DatabaseWrapper {
    pub async fn new(db_host: &String, db_user: &String, db_password: &String) -> DatabaseWrapper {
        // connecting to the database
        let (raw_client, connection) = tokio_postgres::connect(
            format!(
                "dbname=acsim_db hostaddr={} user={} password={}",
                db_host, db_user, db_password
            )
            .as_str(),
            tokio_postgres::NoTls,
        )
        .await
        .unwrap();

        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("connection error: {}", e);
            }
        });

        DatabaseWrapper { client: raw_client }
    }

    fn log_query_status(status: Result<u64, tokio_postgres::error::Error>, operation: &str) {
        match status {
            Ok(v) => log::debug!("{} success: {}", operation, v),
            Err(e) => log::error!("{} failure: {}", operation, e),
        };
    }

    pub async fn count_messages(&self, board: &str) -> Result<i64, Error> {
        match self
            .client
            .query_one(
                "SELECT COUNT(msgid) FROM messages WHERE board=($1)",
                &[&board],
            )
            .await
        {
            Ok(r) => Ok(r.get::<usize, i64>(0)),
            Err(e) => Err(e),
        }
    }

    pub async fn count_submessages(&self, msgid: i64) -> Result<i64, Error> {
        match self
            .client
            .query_one(
                "SELECT COUNT(*) FROM submessages WHERE parent_msg=($1)",
                &[&msgid],
            )
            .await
        {
            Ok(r) => Ok(r.get::<usize, i64>(0)),
            Err(e) => Err(e),
        }
    }

    pub async fn delete_least_active(&self, board: &str) {
        DatabaseWrapper::log_query_status(self.client.execute("DELETE FROM messages WHERE latest_submsg = (SELECT MIN(latest_submsg) FROM messages WHERE board=($1))", &[&board]).await, "Deleting least active message");
    }

    pub async fn get_submessages(&self, msgid: i64) -> Result<Vec<Row>, Error> {
        self.client
            .query("SELECT * FROM submessages WHERE parent_msg=($1)", &[&msgid])
            .await
    }

    pub async fn get_messages(
        &self,
        board: &str,
        page: i64,
        limit: i64,
    ) -> Result<Vec<Row>, Error> {
        self.client
            .query(
                "SELECT * FROM messages WHERE board=($1) ORDER BY latest_submsg DESC OFFSET ($2) LIMIT ($3)",
                &[&board, &page, &limit],
            )
            .await
    }

    pub async fn get_single_message(&self, msgid: i64) -> Result<Row, Error> {
        self.client
            .query_one("SELECT * FROM messages WHERE msgid=($1)", &[&msgid])
            .await
    }

    pub async fn update_message_activity(&self, msgid: i64, since_epoch: i64) {
        DatabaseWrapper::log_query_status(
            self.client
                .execute(
                    "UPDATE messages SET latest_submsg = ($1) WHERE msgid = ($2)",
                    &[&msgid, &since_epoch],
                )
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
        DatabaseWrapper::log_query_status(self.client.execute("INSERT INTO messages(time, author, msg, image, latest_submsg, board) VALUES (($1), ($2), ($3), ($4), ($5), ($6))", &[&time, &author, &msg, &image, &latest_submsg, &board]).await, "Inserting row into messages table");
    }

    pub async fn insert_to_submessages(
        &self,
        parent_msg: i64,
        time: i64,
        author: &str,
        submsg: &str,
        image: &str,
    ) {
        DatabaseWrapper::log_query_status(self.client.execute("INSERT INTO submessages(parent_msg, time, author, submsg, image) VALUES (($1), ($2), ($3), ($4), ($5))", &[&parent_msg, &time, &author, &submsg, &image]).await, "Inserting row into submessages table");
    }
}
