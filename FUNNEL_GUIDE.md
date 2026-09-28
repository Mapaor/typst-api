## Let's go!
No introduction this is extremely straightforward. On your home server simply run:
```
sudo tailscale funnel 8080
```
*Obviously if you set up a different port on the `.env` use that instead of `8080`.*

Now wait about 10 seconds and go visit the provided link. It will be something like `https://machine-name.tailnet-name.ts.net`. 

*Note: You can choose the machine name with `sudo tailscale set --hostname <new-name>` and try to get a nice tailnet name on the Tailscale admin console, in the DNS tab, using the `Rename tailnet...` button.*

You should see the welcome json with the description, version, GitHub link, etc. You can visit this same link from any other device and it will work, your api is now public!

Great so this is it? One simple command and we're done? Almost, we just need to automate the funnel service so that if our home server reboots it will start again automatically.

## Turn it into a service

What we'll do is create a `.service` file inside `/etc/systemd/system` and use `systemd` to activate the service.

1. Create a `tailscale_funnel.service` with:

    ```bash
    sudo nano /etc/systemd/system/tailscale_funnel.service
    ```

2. In there we'll put the following content:
    ```bash
    [Unit]
    Description=Tailscale Funnel to our home server 8080 port
    After=network.target tailscaled.service

    [Service]
    User=root # required
    ExecStart=/usr/bin/tailscale funnel 8080
    Restart=always
    RestartSec=10s

    [Install]
    WantedBy=multi-user.target
    ```

3. And now we enable (and also start) the service

    ```bash
    sudo systemctl daemon-reload
    sudo systemctl enable tailscale_funnel.service
    sudo systemctl start tailscale_funnel.service
    ```

    *Note: you can check the state of all the services with `sudo systemctl list-unit-files --type=service`. You can check the status of a single service with `sudo systemctl status tailscale_funnel.service` and you can see the full logs of a service with `sudo journalctl -u tailscale_funnel.service`. Also, remember to use `sudo systemctl daemon-reload` every time you modify the contents of a running service.*

## And that's it!

You can now reboot your home server, and after a few minutes your public API URL should be available.

## What if I have other services running on my host?

The only caveat of this method is that you can only run a maximum of 3 tunnels, one on port 443 (default), one on port 8443, and one on port 10000. If you have other services, that is other ports of the host you want to expose, you need to explicitly tell Tailscale which of the 3 ports to use. For example:
```
sudo tailscale funnel --https=8443 8085
```
The API will be available at the same URL but this time having to specify the port, that is: (`https://hostname.tailnet-name.ts.net:8443/health`).

## What if I want my own domain?

Then check [`CUSTOM_DOMAIN_GUIDE.md`](CUSTOM_DOMAIN_GUIDE.md).