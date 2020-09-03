# Installing rangepoll

Once you've downloaded and compiled **rangepoll**, it might be interesting to run the software as a service on your server.
This document lists the steps required to run rangepoll as a local (non public) HTTP server only listening on local network and port 3001.

You'll need to set up a reverse proxy for SSL (using NGINX is recommended because it's very simple) so only the public HTTP server is listening to the wild wide web.

On a Debian based system, you'll need to:

## Create right limited user rangepoll

```
$ sudo adduser rangepoll --disabled-login --disabled-password --shell /bin/false --no-create-home
```

## Create a directory for storing rangepoll's files

```
$ sudo mkdir -p /var/lib/rangepoll/voters
$ sudo mkdir -p /var/lib/rangepoll/polls
$ sudo cp -a rangepoll/static rangepoll/templates rangepoll/voters /var/lib/rangepoll
$ sudo cp rangepoll/target/release/rangepoll /var/lib/rangepoll/
$ sudo chown -R rangepoll:rangepoll /var/lib/rangepoll 
```

Then let's build a configuration for your server:
```
$ (cd /var/lib/rangepoll && sudo -u rangepoll /var/lib/rangepoll/rangepoll) 
# Fill your server details: base_url should contain the public facing URL 
$ sudo -u rangepoll nano /var/lib/rangepoll/config.yml
$ sudo chown rangepoll:rangepoll /var/lib/rangepoll/config.yml
$ sudo chmod 0400 /var/lib/rangepoll/config.yml
```

Then create a secret.txt file for tokens (store any random sentence in there only you know about)
$ sudo -u rangepoll nano /var/lib/rangepoll/secret.txt
$ sudo chmod 0400 /var/lib/rangepoll/secret.txt


## Create a systemd service:

```
$ sudo cp rangepoll/rangepoll.service /lib/systemd/system/rangepoll.service
$ sudo systemctl enable rangepoll
$ sudo systemctl start rangepoll
```

## Create a NGINX reverse proxy for this service

```
$ sudo su
# cat > /etc/nginx/sites-available/your.server.com << EOF
server {
    access_log /var/www/poll/logs/access.log;
    error_log /var/www/poll/logs/error.log;

    client_max_body_size 128M;

    location / {
       proxy_pass http://127.0.0.1:3001;
       proxy_set_header Host $host;
       proxy_set_header X-Real-IP $remote_addr;
       proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
       proxy_set_header X-Forwarded-Proto $scheme;
    }

    server_name your.server.com;
}
EOF
# ln -sf /etc/nginx/sites-available/your.server.com /etc/nginx/sites-enabled/your.server.com
# systemctl restart nginx
# certbot --nginx -d your.server.com
```

**Congratulations, you have your own rangepoll server running!**