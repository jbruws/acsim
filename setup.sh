# Setting up environment for the server
echo 'Creating directory for user images'
mkdir ./user_images

echo 'Creating default config file for server'
echo '
{
	"db_host": "127.0.0.1",
	"db_user": "$1",
	"db_password": "change_this",
	"server_ipv4": "127.0.0.1",
	"server_ipv6": "::1",
	"server_port": 8080,
	"bind_to_one_ip": false,
	"deletion_timer": 600,
	"bumplimit": 200,
	"soft_limit": 100,
	"hard_limit": 125,
	"site_name": "ASCIM",
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
echo 'Creating postgres database'
createdb -U $1 acsim_db 

echo 'Creating table scheme'
echo 'CREATE TABLE IF NOT EXISTS messages (
		msgid BIGSERIAL PRIMARY KEY,
		time BIGINT NOT NULL,
		author VARCHAR (255) NOT NULL,
		msg VARCHAR (4096) NOT NULL,
		image VARCHAR (128),
		latest_submsg BIGINT,
		board VARCHAR (16) NOT NULL
	);
	CREATE TABLE IF NOT EXISTS submessages (
		parent_msg BIGINT NOT NULL,
		time BIGINT NOT NULL,
		author VARCHAR (255) NOT NULL,
		submsg VARCHAR (4096) NOT NULL,
		image VARCHAR (128)
	);' | psql -U $1 -d acsim_db;

echo 'Success'
