# qibe - Quick ImageBoard Engine

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

1. After installation, user `postgres` should be created automatically. Change its password:
    
    `# passwd postgres *[password_here]*`
    
    Then `su` into it and start `psql`:
    
    `# su - postgres`
    
    `$ psql`
    
2. You are now in the postgresql shell. Create a database named `qibe_db` and connect to it:
    
    `CREATE DATABASE qibe_db;`
    
    `\c qibe_db`
    
3. Create relevant tables:
    
  ```
      qibe_db=# CREATE TABLE messages (
      qibe_db=#  msgid BIGINT PRIMARY KEY,
      qibe_db=#  time BIGINT,
      qibe_db=#  author VARCHAR (255),
      qibe_db=#  latest_submsg BIGINT,
      qibe_db=#  image VARCHAR (128),
      qibe_db=#  msg VARCHAR (4096)
      qibe_db=# );
  ```
    
  ```
      qibe_db=# CREATE TABLE submessages (
      qibe_db=#  parent_msg BIGINT,
      qibe_db=#  time BIGINT,
      qibe_db=#  author VARCHAR (255),
      qibe_db=#  image VARCHAR (128),
      qibe_db=#  submsg VARCHAR (4096)
      qibe_db=# );
  ```
    
4. Now, you can enter `exit` twice to exit `psql` and `su`, then proceed to the next step.

### 3. QIBE configuration

1. Clone the repository:
    
    `git clone https://github.com/jbruws/qibe.git`
    
    `cd qibe`
    
2. Once you're in the correct directory, run the `setup.sh` script.
    
    `./setup.sh`
    
    Open the `config.json` file and change the `db_password` entry to include password to `postgres` user you set in 1.2.
    
3. Finally, run the program:
    
    `cargo run`
    
Once the compilation finishes, application logs will start appearing in the console and in `qibe.log` file. Navigate to `127.0.0.1:8080` in your browser, and you should be greeted with QIBE's home page. By default, the server will be accessible with any IP (`0.0.0.0`), **as long as the port 8080 is open.**
