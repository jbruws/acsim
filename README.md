# `acsim` - Asynchronous Simple Imageboard

Basic imageboard engine written in `rust` and `actix-web`.

Features:

- Written in Rust (blazingly fast btw) and completely JS-free by default
- Website frontend is completely data-driven; you can change it in any way (as long as you keep the handlebars templates and general file structure)
- Supports both SQLite and Postgres databases  

# Installation

## Standalone Docker container

A Docker image is available for ACSIM.

`# docker run --net=host -e ACSIM_PASS_OVERRIDE=YOURADMINPASSWORD --mount source=acs_data,target=/acsim/data --mount source=acs_frontends,target=/acsim/frontends jbruws/acsim:latest`

This command will set up an SQLite-based instance with two Docker volumes for web frontend and board backend data, as well as a unique password for admin dashboard (available at `127.0.0.1:8080/dashboard`). This is probably the fastest way to spin up an instance.

## Manual installation

1. Install the dependencies:

    - `cargo`
    - `git`
    - `sqlite3` (if you will use SQLite database)
    - `postgresql` (if you will use Postgres database)
    - `libssl-dev`
    - `libmagic-dev`

2. Clone the repository and enter it:
    
   `git clone https://github.com/jbruws/acsim.git`
    
   `cd acsim`

   **DO NOT** change directory afterwards, as this will break relative paths in `setup.sh` script. Just launch everything from the `acsim` directory.
    
4. Once you're in the `acsim` directory, run the `setup.sh` script, either with `SQLITE` argument and no username or `POSTGRES` argument and username you wish to use to connect to the database. Here's examples:

    - `./setup.sh POSTGRES postgres`
    
    - `./setup.sh SQLITE`

    You'll be prompted for a admin password. It's stored as a hash, so don't forget it! 

    View `.env` and `data/config.yaml` files and check them for any errors.
    
5. Finally, run the program:
    
    `cargo run`
    
Once the compilation finishes, application logs will start appearing in the console and in `data/acsim.log` file. Navigate to `127.0.0.1:8080` in your browser, and you should be greeted with ACSIM's home page. By default, the server will be accessible with any IP (`0.0.0.0`), **as long as the port 8080 is open.**

# Special Thanks

[@ZueffC](https://github.com/ZueffC) - testing, coding advice

[@CppCoder1](https://github.com/CppCoder1), [@Befrimon](https://github.com/Befrimon) - testing
