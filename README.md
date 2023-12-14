# `acsim` - Asynchronous Simple Imageboard

Basic imageboard engine written in `rust` and `actix-web`. Lightweight and completely JS-free.

The engine is still in active development. Expect some bugs, missing features and drastic changes in design.

## Installation

Here's the dependency list:

- `cargo`
- `git`
- `sqlite3` (if you will use SQLite database)
- `postgresql` (if you will use Postgres database)

### Manual installation
1. Clone the repository:
    
    `git clone https://github.com/jbruws/acsim.git`
    
    `cd acsim`
    
2. Once you're in the `acsim` directory, run the `setup.sh` script, either with `SQLITE` argument and no username or `POSTGRES` argument and username you wish to use to connect to the database. Here's examples:

    `./setup.sh POSTGRES postgres`
    
    `./setup.sh SQLITE`

    View the `.env` file and check it for any errors.
    
4. Finally, run the program:
    
    `cargo run`
    
Once the compilation finishes, application logs will start appearing in the console and in `acsim.log` file. Navigate to `127.0.0.1:8080` in your browser, and you should be greeted with ACSIM's home page. By default, the server will be accessible with any IP (`0.0.0.0`), **as long as the port 8080 is open.**

## Special Thanks

[@ZueffC](https://github.com/ZueffC) - testing, coding advice

[@CppCoder1](https://github.com/CppCoder1), [@Befrimon](https://github.com/Befrimon) - testing
