mkdir ./user_images
touch config.json
echo '
{
	"db_host": "127.0.0.1",
	"db_user": "postgres",
	"db_password": "change_this",
	"server_ip": "127.0.0.1",
	"server_port": 8080,
	"bind_to_one_ip": false
}
' > config.json
