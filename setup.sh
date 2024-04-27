#!/bin/bash

# Setting up environment for the server

if [ ! -d "./data" ]; then
	echo 'Creating directory for persistent data'
	mkdir ./data
fi

if [ ! -d "./data/user_images" ]; then
	echo 'Creating directory for user images'
	mkdir ./data/user_images
fi

if [ ! -d "./data/captcha" ]; then
	echo 'Creating directory for temporary captcha storage'
	mkdir ./data/captcha
fi

if [ ! -f "./data/banlist.yaml" ]; then
	echo 'Creating empty banword list'
	echo '---
# This is a global list of banned words in YAML format.
# You can enter actual words or Regex wrapped in single quotes.
# Format the file as a YAML list.
' > ./data/banlist.yaml
fi

# if not running in container, prompt for dashboard password
if [ -z "${acsim_compose}" ] && [ -z "${acsim_docker}" ]; then
	echo -n "Enter the password that will be used for admin dashboard: "
	read acsim_pass
else
	acsim_pass=$(cat /dev/random | head -c 20 | sha256sum | head -c 8 | xargs)
fi

passhashed=$(echo -n $acsim_pass | sha256sum | head -c 64 | xargs)

if [ ! -f "./data/config.yaml" ]; then
	echo 'Creating default config file for server'
	echo "---
# IP address and port used to serve the imageboard itself
bind_addr: 127.0.0.1
bind_port: 8080

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

# Max number of captcha images in ./data/captcha
# When it is reached, least used images will be deleted
# 200 images is about ~7.5 megabytes
captcha_num_limit: 200

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
	if [ -z $2 ]; then
		echo "Please specify username for Postgres DB"
		exit
	fi
	# if not in Compose, create the db and tables
	if [ -z "${acsim_compose}" ]; then
		echo 'Creating database'
		createdb -U $2 acsim_db
		echo 'Creating table scheme'
		echo ./pg_init.sql | psql -U $2 -d acsim_db;
		pg_connect_string="DATABASE_URL=\"postgres://$2@localhost:5432/acsim_db\""
	# otherwise, leave db/table creation to Postgres image
	else
		echo 'Database setup rests on Compose image'
		pg_connect_string="DATABASE_URL=\"postgres://postgres:generic@db:5432/acsim_db\""
	fi
	echo "Writing database URL to .env"
	echo $pg_connect_string > .env

elif [ "$1" = "SQLITE" ]; then
	echo "Creating database"
	echo "Creating table scheme" # same thing here
	sqlite3 ./data/acsim.db < ./sqlite_init.sql
	echo "Writing database URL to .env"
	echo 'DATABASE_URL="sqlite://data/acsim.db"' > .env
else
	echo "Please specify database type (POSTGRES or SQLITE) in script args"
	exit
fi

echo 'Success'
