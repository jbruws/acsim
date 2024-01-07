# ACSIM Changelog

## v0.10.0

- Added a rate limiter to (hopefully) prevent board wipes
- Added message content check to (hopefully) prevent board wipes
- Added error pages
- Added `vim` to ACSIM Docker image
- Temp files that are not processed by the engine are now automatically deleted
- Failed attempts to delete or copy temp files are now logged
- Moved server data (such as config file and user images) to `./data` directory
- Fixed SQL query for message selection on a board

## v0.11.0

- Links now have underlines (again)
- Changed link color
- Fixed footers not being the width of the entire screen
- Fixed formatter displaying unnecessary line breaks
