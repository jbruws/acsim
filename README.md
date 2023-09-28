# `qibe` - Quick ImageBoard Engine

Basic message board engine written in Rust and Actix Web.

# `config.json` file

`"db_host"`: IP of the server where `qibe_db` database is hosted.

`"db_user"`: User which is used to log into the database. Usually it's the default, `postgres`.

`"db_password"`: Password for the DB.

`"server_ipv4"`: IPv4 address of the web server.

`"server_ipv6"`: IPv6 address of the web server.

`"server_port"`: Port which is used for serving pages.

`"bind_to_one_ip"`: Only bind to IPs specified in `server_ipv4` and `server_ipv6` instead of binding to all available addresses

`"deletion_timer"`: After this many seconds, the server will delete the least active topic (if number of topics on a given board is above `soft_limit`)

`"bumplimit"`: After this many submessages, a topic stops updating its `latest_submsg` field (becoming inactive in the server's eyes)

`"soft_limit"`: Number of messages on a board required to attempt inactive topic deletion every `deletion_timer` seconds.

`"hard_limit"`: If the number of messages on a board exceeds this number, board's least active topic will be deleted when a new one is submitted.

`"boards"`: Dictionary (`HashMap`) containing letters (board designations) and their main topics.

`"taglines"`: List of phrases randomly displayed in board header. Usually humourous. Feel free to put whatever you want here.

# Installation
## Manual
#### 1. PostgreSQL Installation
1. Debian/Ubuntu-based systems:
    
    `# apt install postgresql`

2. Arch-based systems:
    
    `# pacman -S postgresql`
    
3. Then, start and enable the service.

    *If you use OpenRC as your init system, install `postgresql-openrc` for service scripts!*
    
    `# systemctl enable postgresql && systemctl start postgresql`
    
    or
    
    `# rc-update add postgresql default && rc-service postgres start`

#### 2. PostgreSQL Configuration

1. After installation, enter the Postgres shell through `psql`:
    
    `$ psql -U postgres`
    
2. Next, create a database named `qibe_db`:
    
    `postgres=# CREATE DATABASE qibe_db;`
    
3. Now, you can `exit` from `psql`, then proceed to the next step.

### 3. QIBE configuration

1. Clone the repository:
    
    `git clone https://github.com/jbruws/qibe.git`
    
    `cd qibe`
    
2. Once you're in the `qibe` directory, run the `setup.sh` script.
    
    `./setup.sh`

3. OPTIONAL: you can add password protection to Postgres. This will be especially useful if you're running Postgres on a machine with multiple users. Enter the shell:

    `psql -U postgres`

    and change the password for user `postgres`:

    `postgres=# \password`

    A prompt will appear. Enter the new password twice and `exit` from the shell.

    Edit the `config.json` file from the `qibe`'s project directory to include the new password in the `db_password` field.

    Finally, you need to edit the `/var/lib/postgres/data/pg_hba.conf` and replace `trust` (in the `METHOD` field) with `md5` in all lines where it's present.

    Now your Postgres installation requires a password for interacting with databases. Qibe will use the password from the config for connection.
    
5. Finally, run the program:
    
    `cargo run`
    
Once the compilation finishes, application logs will start appearing in the console and in `qibe.log` file. Navigate to `127.0.0.1:8080` in your browser, and you should be greeted with Qibe's home page. By default, the server will be accessible with any IP (`0.0.0.0`), **as long as the port 8080 is open.**
