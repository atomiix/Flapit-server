# Flapit Server

This project is the result of the reverse engineering of the communication between the Flapit device and the Flapit API.

## ⚠️ DNS workaround

Because the Flapit device has an hardcoded endpoint (hub.flapit.com), we need to find a way to redirect the endpoint to our custom server.

As I’m already running [pihole](https://pi-hole.net/) on my local network, I added a custom DNS entry that redirect hub.flapit.com to the IP address flapit_server is running on.

I think it could be possible to use a Raspberry Pi or something equivalent (even maybe an ESP32?) to create a Wifi Access Point for the Flapit device to connect to, and add an entry in `/etc/hosts` to make the redirection but I never tried it.

## How to use

Download the latest binary according to your OS/processor architecture:

`$ wget https://github.com/atomiix/Flapit-server/releases/download/v0.0.3/x86_64-unknown-linux-gnu.zip`

Unzip the binary:

`$ unzip x86_64-unknown-linux-gnu.zip`

Copy the binary to `/usr/local/bin`:

`$ sudo cp flapit_server /usr/local/bin`

#### Commands

By default, the Flapit API server will be listening on port 443 and will listen for devices on port 3000.

You can change this with command arguments:

`$ flapit_server --device-port=4443 --api-port=3333`

If you want to enable verbose log, you can use the --verbose/-v option:

`$ flapit_server -v`

#### Running in background

On linux with systemd, you can create a `flapit.service` file like so:
```service
[Unit]
Description=Flapit server
After=network.target

[Service]
Type=simple
User=pi
Group=pi

ExecStart=/usr/local/bin/flapit_server --device-port=4443 -v

Restart=on-failure
RestartSec=3

[Install]
WantedBy=multi-user.target
```

Copy it to `/etc/systemd/system/`:

`$ sudo cp flapit.service /etc/systemd/system/`

And start it:

`$ sudo systemctl start flapit`

### Use the API

`$ curl -d device=FLP1-1234567890 -d message=hello http://flapitserverip:3000`

The `message` format is the same as described in the [Flapit API documentation](https://www.flapit.com/en/api.html)
