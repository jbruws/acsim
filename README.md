# `acsim` - Asynchronous Simple Imageboard

Basic imageboard engine written in `rust` and `actix-web`. Lightweight and completely JS-free.

The engine is still in active development. Expect some bugs, missing features and drastic changes in design.

## Installation
### Manual
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

#### 2. ACSIM configuration

1. Clone the repository:
    
    `git clone https://github.com/jbruws/acsim.git`
    
    `cd acsim`
    
2. Once you're in the `acsim` directory, run the `setup.sh` script with the username you wish to use to create and modify databases.
    
    `./setup.sh [postgres_username]`

3. OPTIONAL: you can add password protection to Postgres. This will be especially useful if you're running Postgres on a machine with multiple users. Enter the shell:

    `psql -U [postgres_username]`

    and change the password for your user:

    `[postgres_username]=# \password`

    A prompt will appear. Enter the new password twice and `exit` from the shell.

    Edit the `config.json` file from the `acsim`'s project directory to include the new password in the `db_password` field.

    Finally, you need to edit the `/var/lib/postgres/data/pg_hba.conf` and replace `trust` (in the `METHOD` field) with `md5` in all lines where it's present.

    Now your Postgres installation requires a password for interacting with databases. ACSIM will use the password from the config for connection.
    
5. Finally, run the program:
    
    `cargo run`
    
Once the compilation finishes, application logs will start appearing in the console and in `acsim.log` file. Navigate to `127.0.0.1:8080` in your browser, and you should be greeted with ACSIM's home page. By default, the server will be accessible with any IP (`0.0.0.0`), **as long as the port 8080 is open.**

## `config.yaml` file breakdown

- `db_host`: IP of the server where `acsim_db` database is hosted.

- `db_user`: User which is used to log into the database. Usually it's the default, `postgres`.

- `db_password`: Password for the DB.

- `server_ipv4`: IPv4 address of the web server.

- `server_ipv6`: IPv6 address of the web server.

- `server_port`: Port which is used for serving pages.

- `bind_to_one_ip`: Only bind to IPs specified in `server_ipv4` and `server_ipv6` instead of binding to all available addresses

- `bumplimit`: After this many submessages, a topic stops updating its `latest_submsg` field (becoming inactive in the server's eyes)

- `hard_limit`: If the number of messages on a board exceeds this number, board's least active topic will be deleted when a new one is submitted.

- `site_name`: Name of your site in general, displayed in `<title>` tags on pages.

- `site_frontend`: Name of the frontend (page/template structure) used by ACSIM. Refer to `frontends/acsim_ungapped` for reference.

- `page_limit`: How many messages are displayed per page.

- `boards`: Dictionary (`BTreeMap`) containing letters (board designations) and their main topics, sorted alphabetically by board name.

- `taglines`: List of phrases randomly displayed in board header. Usually humourous. Feel free to put whatever you want here.

## Special Thanks

[@ZueffC](https://github.com/ZueffC) - testing, coding advice

[@CppCoder1](https://github.com/CppCoder1), [@Befrimon](https://github.com/Befrimon) - testing
