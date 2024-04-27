# ACSIM Changelog

Changelog may get imprecise in earlier versions, since I started writing it at version 0.10; apologies in advance for any inaccuracies (although it hardly matters for early versions).

## v1.1.0 - 22.04.2024

### Added

- Added a function that blocks messages if it detects a banned word
- Added simple CAPTCHA for message sending and reporting
- Added a Docker Compose config and appropriate images
- Added dashboard password override environment variable

### Fixed

- `purge_images` no longer attempts to delete empty filepaths
- Fixed images not being deleted when threads are removed due to board limit
- Fixed false positive error messages when deleting media files
- Fixed autoincrement issue with `flagged_messages` in Postgres

### Changed

- Split SQL commands from `setup.sh` into separate scripts
- Links to other messages no longer require the user to specify a board
- Improved documentation comments in code (somewhat)
- Removed unneeded `Not` implementation on `FileType` enum
- Admins are now redirected back to the relevant dashboard page when deleting messages

### Removed

- Removed separate config options for binding IPv4 and IPv6 addresses

## v1.0.0 - 08.03.2024

### Added

- Added a config option for displaying or hiding log level
- Added an admin dashboard with message deletion capabilities

### Fixed

- Fixed catalog page not displaying properly
- Fixed broken message search due to incorrect query

### Changed

- Changed message header formatting to a more traditional style
- CSS styles for message blocks are split into a separate file
- Head message links no longer lead back to the board
- Board message links now open a new tab when clicked
- Submessage indices are now recorded in DB
- Gaps between messages on boards are now larger
- Removed the awful, redundant match exressions from `src/db_control.rs`
- Added more etymological clarity

## v0.11.0 - 31.01.2024

### Added

- Links now have underlines (again)
- Retroactively added `CHANGELOG.md` entries for versions before 0.10
- ACSIM now looks in `~/.local/share/acsim` if no server data is found in `.`

### Fixed

- Fixed footers not being the width of the entire screen
- Fixed formatter displaying unnecessary line breaks in formatted messages
- (Hopefully) fixed broken page structures when resizing in Safari
- Spam protection by message comparison is now applied to submessages as well
- Fixed several Dependabot alerts

### Changed

- Changed link color
- Board list on the main page now appears in the order specified in config file instead of being alphabetically sorted
- Log format now includes the module "issuing" the log
- Logging of debug data is now configurable in `config.yaml`
- ACSIM now panics when logger initialization fails
- `get_nth_most_active` method has been renamed to `get_last_message` and had its functionality changed accordingly
- Cleaned up `formatting_rules.yaml`
- `acsim_ungapped` now has higher color contrast
- Restructured `CHANGELOG.md`
- Improved page layout on mobile devices

## v0.10.0 - 03.01.2024

### Added

- A rate limiter to (hopefully) prevent board wipes
- Message content check to (hopefully) prevent board wipes
- HTTP error pages
- `vim` to ACSIM Docker image

### Fixed

- Temp files that are not processed by the engine are now automatically deleted
- Failed attempts to delete or copy temp files are now logged
- Fixed SQL query for message selection on a board

### Changed

- Moved server data (such as config file and user images) to `./data` directory

## v0.9.0 - 14.12.2023

### Added

- Created a Docker image for ACSIM
- Added support for SQLite databases
- Added more options for message formatting (headers/horizontal rulers)
- Images are now magnified if you press+hold LMB on them

### Fixed

- Page footer is now always at the bottom of the page, regardless of contents of the page (courtesy of ZueffC)
- Fixed typo in `setup.sh`
- Updated `openssl` crate to get rid of a bug in its earlier version
- Fixed message sage
- Partially fixed bug with busted catalog page due to cut-off HTML tags

### Changed

- Split image processing (for message formatting) into a separate method
- Changed database library from `tokio-postgres` to `sqlx`
- Moved database URL to `.env` file (`sqlx` requires it, see above)
- ACSIM now accepts trailing slashes in URLs

## v0.8.0 - 08.11.2023

### Added

- Added ability to sage posts in order to prevent activity update when replying
- Added formatting for spoilers (|| ||)
- Sent/received data is now GZip'ped by default

### Changed

- Rules for message formatting are now contained in a separate file (`formatting_rules.yaml`)
- Changed index page formatting for better readability
- HTML formatting methods now accept `Row` objects instead of separate variables

## v0.7.0 - 04.11.2023

### Added

- Added catalog pages for boards

### Changed

- Renamed some CSS files for clarity
- Moved config explanation comments into the config itself (they used to be in `README.md`
- Split `routes.rs` into several modules, according to which page they serve

## v0.6.0 - 26.10.2023

### Added

- Added an index page with a list of boards
- Added HTTPS support via `openssl`
- Added more formatting options (strikethrough text and hyperlinks)
- Added video upload capabilities (only up to 40 MB though)
- Added 'Special Thanks' section in `README.md`
- Images are now magnified when you hover over them

### Fixed

- Users can no longer send empty messages

### Changed

- Username is no longer required in messages; empty usernames are replaced with "Anonymous"

## v0.5.0 - 19.10.2023

### Added

- Images sent by users are now validated using `magic`

### Fixed

- Fixed `setup.sh` to look for YAML file instead of JSON

### Changed

- Users can now send several files in one message

### Removed

- Removed default `acsim_base` frontend in favor of `acsim_ungapped`

## v0.4.0 - 15.10.2023

### Added

- Added new `acsim_ungapped` frontend

### Changed

- Changed license back to BSD 3-Clause from GPLv3
- Config is now in YAML instead of JSON
- Shrank database fields for additional protection against excessively long messages

## v0.3.0 - 11.10.2023

### Fixed

- Fixed links from topic head messages/submessages to board messages

### Changed

- All message formatting (including message links) is now done with Regex
- Replaced hyphens in filenames/directory names with underscores
- Changed some CSS colors and variable names
- Board page number is now passed as a GET parameter

### Removed

- Removed unused 'soft limit' functionality (I couldn't get it to work with Tokio)

## v0.2.0 - 07.10.2023

### Added

- Added some documentation comments for main modules
- ACSIM frontend is now modular (decouples from backend and stored separately)
- Added basic Markdown-like message formatting

### Changed

- Changed license from BSD 3-Clause to GPLv3

## v0.1.0 - 06.10.2023

### Added

- Initial version
- Setup script (`setup.sh`) that creates database, tables and config
- Logging with `fern`
- Basic image upload functionality
- Message/submessage (reply) system
- HTML tag filtering
- Links to other board messages
- Hard and soft limit for number of messages (latter not implemented)
- Basic CSS specifically for mobile devices
- Bindings to IPv4 and IPv6 addresses

### Changed
    
- Renamed project twice
- Split messages into several boards
