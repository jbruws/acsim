# `qibe` - Quick ImageBoard Engine

Basic message board engine written in Rust and Actix Web.

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
