# Setting up environment for the server
if [ ! -d ./user_images ]; then
	echo 'Creating directory for user images'
	mkdir ./user_images
fi

if [ ! -f "config.json" ]; then
	echo 'Creating default config file for server'
	echo "
---
db_host: 127.0.0.1
db_user: $1
db_password: change_this
server_ipv4: 127.0.0.1
server_ipv6: ::1
server_port: 8080
bind_to_one_ip: false
bumplimit: 200
hard_limit: 125
site_name: ACSIM
site_frontend: acsim_ungapped
page_limit: 25
boards:
    b: Random
    s: Software
    ca: Cryptoanarchy
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
		image VARCHAR (127),
		latest_submsg BIGINT,
		board VARCHAR (15) NOT NULL
	);
	CREATE TABLE IF NOT EXISTS submessages (
		parent_msg BIGINT NOT NULL,
		time BIGINT NOT NULL,
		author VARCHAR (63) NOT NULL,
		submsg VARCHAR (4095) NOT NULL,
		image VARCHAR (127),
		CONSTRAINT bind_msg
			FOREIGN KEY(parent_msg)
				REFERENCES messages(msgid)
				ON DELETE CASCADE
	);' | psql -U $1 -d acsim_db;

echo 'Success'
