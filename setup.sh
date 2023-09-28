# Setting up environment for the server
mkdir ./user_images
touch config.json
echo '
{
	"db_host": "127.0.0.1",
	"db_user": "postgres",
	"db_password": "change_this",
	"server_ipv4": "127.0.0.1",
	"server_ipv6": "::1",
	"server_port": 8080,
	"bind_to_one_ip": false,
	"deletion_timer": 600,
	"bumplimit": 200,
	"soft_limit": 100,
	"hard_limit": 125,
	"boards": {
		"b": "Random", 
		"s": "Software",
		"ca": "Cryptoanarchy"
	},
	"taglines": [
		"you should back your data up NOW!!!",
		"In Rust We Trust"
	]
}
' > config.json

# Creating database tables
echo 'CREATE TABLE IF NOT EXISTS messages (
		msgid SERIAL PRIMARY KEY,
		time BIGINT NOT NULL,
		author VARCHAR (255) NOT NULL,
		board VARCHAR (16) NOT NULL,
		latest_submsg BIGINT,
		image VARCHAR (128),
		msg VARCHAR (4096) NOT NULL
	);
	CREATE TABLE IF NOT EXISTS submessages (
		parent_msg BIGINT NOT NULL,
		time BIGINT NOT NULL,
		author VARCHAR (255) NOT NULL,
		image VARCHAR (128),
		submsg VARCHAR (4096) NOT NULL
	);' | psql -U postgres -d qibe_db;
