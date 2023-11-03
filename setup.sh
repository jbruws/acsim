# Setting up environment for the server
if [ ! -d ./user_images ]; then
	echo 'Creating directory for user images'
	mkdir ./user_images
fi

if [ ! -f "config.yaml" ]; then
	echo 'Creating default config file for server'
	echo "
---
# IP address of the server where the database is hosted
db_host: 127.0.0.1

# Postgres user used to access the database
db_user: $1

# Password for the database (only used if you enabled it. Refer to README)
db_password: change_this

# IP addresses and port used to serve the imageboard itself
server_ipv4: 127.0.0.1
server_ipv6: ::1
server_port: 8080

# If set to true, only binds to addresses specified above, otherwise binds to 0.0.0.0
bind_to_one_ip: false

# Use HTTPS. Only enable if you have cert.pem and key.pem in keys/ directory in project root!
use_https: false

# Bumplimit for posts. If number of replies reaches this, post activity stops being updated
bumplimit: 200

# Max amount of messages on one board
hard_limit: 125

# Name of the imageboard displayed to users
site_name: ACSIM

# Frontend used by the imageboard
site_frontend: acsim_ungapped

# Limit for number of messages on one page
page_limit: 20

# Boards served to users. Consists of board designation and short description
boards:
    b: Random
    s: Software
    ca: Cryptoanarchy

# Taglines. Put whatever you want here. Use quotation marks if the server refuses to starts afterwards.
taglines:
    - you should back your data up NOW!!!
    - In Rust We Trust
	" > config.yaml
fi

# Creating database tables
echo 'Creating postgres database'
createdb -U $1 acsim_db 

echo 'Creating table scheme'
echo 'CREATE TABLE IF NOT EXISTS messages (
		msgid BIGSERIAL PRIMARY KEY,
		time BIGINT NOT NULL,
		author VARCHAR (63) NOT NULL,
		msg VARCHAR (4095) NOT NULL,
		image VARCHAR (255),
		latest_submsg BIGINT,
		board VARCHAR (15) NOT NULL
	);
	CREATE TABLE IF NOT EXISTS submessages (
		parent_msg BIGINT NOT NULL,
		time BIGINT NOT NULL,
		author VARCHAR (63) NOT NULL,
		submsg VARCHAR (4095) NOT NULL,
		image VARCHAR (255),
		CONSTRAINT bind_msg
			FOREIGN KEY(parent_msg)
				REFERENCES messages(msgid)
				ON DELETE CASCADE
	);' | psql -U $1 -d acsim_db;

echo 'Success'
