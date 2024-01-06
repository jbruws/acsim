# `acsim` - Asynchronous Simple Imageboard

Basic imageboard engine written in `rust` and `actix-web`.

Features:

- Written in Rust (blazingly fast btw) and completely JS-free by default
- Website frontend is completely data-driven; you can change it in any way (as long as you keep handlebars templates)
- Supports both SQLite and Postgres databases  

The engine is still in active development. Expect some bugs, missing features and drastic changes in design.

# Installation

## Docker container

A Docker image is available for ACSIM. You can use it to test the engine or quickly spin up an instance. Run:

`# docker run --net=host jbruws/acsim:0.10`

### Data persistency

You can make message data persist between container restarts by creating a Docker volume (through `# docker volume create your_volume_name`) and running:

`# docker run --net=host --mount source=your_volume_name,target=/usr/local/bin/data jbruws/acsim:0.10`

Server data will be stored in `/var/lib/docker/volumes/your_volume_name/_data`. To change said data, edit the relevant files then restart the container.

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
    
3. Once you're in the `acsim` directory, run the `setup.sh` script, either with `SQLITE` argument and no username or `POSTGRES` argument and username you wish to use to connect to the database. Here's examples:

    - `./setup.sh POSTGRES postgres`
    
    - `./setup.sh SQLITE`

    View `.env` and `config.yaml` files and check them for any errors.
    
4. Finally, run the program:
    
    `cargo run`
    
Once the compilation finishes, application logs will start appearing in the console and in `acsim.log` file. Navigate to `127.0.0.1:8080` in your browser, and you should be greeted with ACSIM's home page. By default, the server will be accessible with any IP (`0.0.0.0`), **as long as the port 8080 is open.**

# Special Thanks

[@ZueffC](https://github.com/ZueffC) - testing, coding advice

[@CppCoder1](https://github.com/CppCoder1), [@Befrimon](https://github.com/Befrimon) - testing
