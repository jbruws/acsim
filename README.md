# `acsim` - Asynchronous Simple Imageboard

Basic imageboard engine written in `rust` and `actix-web`. Lightweight and completely JS-free.

The engine is still in active development. Expect some bugs, missing features and drastic changes in design.

## Installation
### Manual installation
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

## Special Thanks

[@ZueffC](https://github.com/ZueffC) - testing, coding advice

[@CppCoder1](https://github.com/CppCoder1), [@Befrimon](https://github.com/Befrimon) - testing
