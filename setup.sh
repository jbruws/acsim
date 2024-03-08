# Setting up environment for the server

if [ ! -d ./data ]; then
	echo 'Creating directory for persistent data'
	mkdir ./data
fi

if [ ! -d ./data/user_images ]; then
	echo 'Creating directory for user images'
	mkdir ./data/user_images
fi

if [[ -z "${acsim_pass}" ]]; then
	echo -n "Enter the password that will be used for admin dashboard: "
	read acsim_pass
fi
echo $acsim_pass
passhashed=$(echo -n $acsim_pass | sha256sum | head -c 64)

if [ ! -f "./data/config.yaml" ]; then
	echo 'Creating default config file for server'
	echo "
---
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

# Limit for number of messages on one page
page_limit: 20

# Max number of requests (in a row) that one IP can send before being blocked.
# Setting this below tripled page_limit is not recommended.
requests_limit: 90

# Enables debug logs, for example database query logging. Can bloat log files very fast.
log_debug_data: false

# Displays log level at the start of each log line
display_log_level: true

# Password for admin dashboard, stored as a SHA-256 hash
admin_password: $passhashed

# Name of the imageboard displayed to users
site_name: ACSIM

# Frontend used by the imageboard
site_frontend: acsim_ungapped

# Boards served to users. Consists of board designation and short description
boards:
    b: Random

# Taglines. Put whatever you want here. Use quotation marks if the server refuses to starts afterwards.
# If you don't want to use taglines at all, just leave this empty
taglines:
" > ./data/config.yaml
fi

# matching database type argument
if [ "$1" = "POSTGRES" ]; then
	echo 'Creating database'
	createdb -U $2 acsim_db

	echo 'Creating table scheme'
	echo 'CREATE TABLE IF NOT EXISTS messages (
			msgid BIGSERIAL PRIMARY KEY,
			board TEXT NOT NULL,
			time BIGINT NOT NULL,
			author TEXT NOT NULL,
			msg TEXT NOT NULL,
			image TEXT NOT NULL,
			latest_submsg BIGINT NOT NULL
		);
		CREATE TABLE IF NOT EXISTS submessages (
			parent_msg BIGINT NOT NULL,
			submsg_id BIGINT NOT NULL,
			board TEXT NOT NULL,
			time BIGINT NOT NULL,
			author TEXT NOT NULL,
			submsg TEXT NOT NULL,
			image TEXT NOT NULL,
			CONSTRAINT bind_msg
				FOREIGN KEY(parent_msg)
					REFERENCES messages(msgid)
					ON DELETE CASCADE
		);
		CREATE TABLE IF NOT EXISTS flagged_messages (
			entry_id INTEGER PRIMARY KEY,
			msg_type TEXT NOT NULL,
			msgid BIGINT NOT NULL UNIQUE,
			submsg_index BIGINT,
			UNIQUE(msgid,submsg_index),
			CONSTRAINT bind_msg
				FOREIGN KEY(msgid)
					REFERENCES messages(msgid)
					ON DELETE CASCADE
		);' | psql -U $2 -d acsim_db;
	echo "Writing database URL to .env"
	echo "DATABASE_URL=\"postgres://$2@localhost:5432/acsim_db\"" > .env
elif [ "$1" = "SQLITE" ]; then
	echo "Creating database"
	echo "Creating table scheme" # same thing here
	sqlite3 -line ./data/acsim.db 'CREATE TABLE IF NOT EXISTS messages (
			msgid INTEGER PRIMARY KEY,
			board TEXT NOT NULL,
			time BIGINT NOT NULL,
			author TEXT NOT NULL,
			msg TEXT NOT NULL,
			image TEXT NOT NULL,
			latest_submsg BIGINT NOT NULL
		);
		CREATE TABLE IF NOT EXISTS submessages (
			parent_msg BIGINT NOT NULL,
			submsg_id BIGINT NOT NULL,
			board TEXT NOT NULL,
			time BIGINT NOT NULL,
			author TEXT NOT NULL,
			submsg TEXT NOT NULL,
			image TEXT NOT NULL,
			CONSTRAINT bind_msg
				FOREIGN KEY(parent_msg)
					REFERENCES messages(msgid)
					ON DELETE CASCADE
			);
			CREATE TABLE IF NOT EXISTS flagged_messages (
				entry_id INTEGER PRIMARY KEY,
				msg_type TEXT NOT NULL,
				msgid BIGINT NOT NULL,
				submsg_index BIGINT,
				UNIQUE(msgid,submsg_index),
				CONSTRAINT bind_msg
					FOREIGN KEY(msgid)
						REFERENCES messages(msgid)
						ON DELETE CASCADE
			);'
	echo "Writing database URL to .env"
	echo 'DATABASE_URL="sqlite://data/acsim.db"' > .env
else
	echo "Please specify database type (POSTGRES or SQLITE) in script args"
	exit
fi

echo 'Success'
