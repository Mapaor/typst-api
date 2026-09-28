# How to set up `typst-api` on a custom domain using Cloudflare Tunnel and Caddy
## Introduction
The machine where you're running the API server is most probably one of the following:
1. A VPS (Virtual Private Server)
2. A home server (mini pc, raspberry pi, NAS, etc.)
3. Your own computer (laptop, desktop pc, phone, etc.)
4. A PaaS running on the cloud (Railway, Render, fly.io, etc.)

This guide is intended for the case number 2. Specifically when your home server is under cg-nat so it does not have a public IP.

For this case, which is I think the most common for people running self-hostable services you typically have two options
a. Use a VPN-mesh virtual network that does NAT-traversal and allows connecting your devices with your home server (Tailscale, Headscale, NetBird, etc.).
b. Use a reverse tunnel service to connect your home server to the public internet (Cloudflared, Pangolin, Tailscale Funnel, Rathole, etc.).

With option A only you from your selected devices can access the API, and it will always be over http, which means flagged as insecure by the browser unless you configure on each device the certificate of your home server. This is perfectly fine for most users but I haven't made a guide for it because it needs no additional configuration other than installing the virtual network service (eg: Tailscale) and knowing your home server's virtual IP.

Option B is a bit more tricky, I'll name what are the most common cases:
i. You buy a domain, manage its DNS records via Cloudflare dashboard and set up Cloudflare Tunnel
ii. You buy a domain and a cheap VPS, manage its DNS from wherever you like (can be Cloudflare as well), point the subdomain wildcard (`*`) to your VPS's IP and install Pangolin there. Pangolin then acts as Cloudflare Tunnel to your home server.
iii. You use Tailscale Funnel and expose your service to a pre-given subdomain of `ts.net`. You get your server exposed but you can't choose your domain.
iv. You use any other reverse tunnel software (like Rathole) and configure it to fit your exact needs (advanced).

Of these, I personally think the most straightforward is option III, here's the guide for it: [`FUNNEL_GUIDE.md`](FUNNEL_GUIDE.md).

This guide is meant for option I. Options II and IV don't have a guide because they are a bit more advanced and they are way too configurable to create a simple to follow guide.

All right, so this is it right? A home server under cg-nat, an owned domain and Cloudflare Tunnel, nothing more right? Almost, allow me to continue...

We are missing the fact that we probably have lots of services running on our home server. To properly handle them all it is very convenient to have a reverse proxy. This allows us to expose only one (or a few) ports of our home server (instead of exposing all our home server's ports where there's a service running to the public internet), and using the reverse proxy to direct them to the proper ports.

The most popular reverse proxies are Nginx, Apache, Traefik and Caddy. Nginx is probably the more powerful, but Caddy is the simplest to set up that gets the job done.

Then your host (home server) could be any machine with any OS. To make things simple we are only going to consider Debian-based operating systems, like Ubuntu Server or Pi OS Lite. 

Finally you could choose to run `typst-api` as a process (the binary running) or as a container, we are gonna focus on the latter, more specifically we are gonna use the Docker Compose option.

With regards to architecture, using Docker allows us to not worry about it because the image this repo publishes to DockerHub is already multi-platform (support both `amd64` and `arm64`).

So, as a wrap, in this guide we will learn precisely that: How to set up Caddy and Cloudflare Tunnel so that your machine under cg-nat can run `typst-api` in a docker container and publish it at a custom domain.

## Pre-requisites
- Curl (`sudo apt install curl -y`)
- Docker ([official guide](https://docs.docker.com/engine/install/ubuntu/#install-using-the-repository))
- A domain with DNS managed by Cloudflare

## Setting it up locally
1. First we obviously log into our home server, I'll assume it's a bash terminal and you're logged-in as a non-root user.
2. We create a folder wherever we want, I like to have my docker projects inside a `docker` folder so I would do:
    ```
    mkdir -p ~/docker/typst-api
    cd ~/docker/typst-api
    ```
3. Now we are gonna create two files: `.env` and `docker-compose.yml`; and an empty folder `fonts/`. We can do it with:
    ```
    touch .env && touch docker-compose.yml && mkdir fonts
    ```
    Next we are gonna use a text editor to edit those 2 files, common options include nano and vim, but I personally like `micro` for its simplicity and ease of use, if you also want to use micro you can install it with `sudo apt install micro`.
4. Let's go see the [`.env.example`](.env.example) and copy its contents into our `.env`. Modify the port if you already have a service running at port `8080` and then save the file.
   ```
   micro .env
   ```
   Save with Ctrl+S and exit with Ctrl+Q.
5. Now let's check [`docker-compose.yml`](docker-compose.yml) and copy its contents to our local `docker-compose.yml` file.
   ```
   micro docker-compose.yml
   ```
6. Finally let's spin-up the container with:
   ```
   docker compose up -d
   ```
7. And check its health status with:
    ```
    curl -X GET http://your-server-ip-here:8080/health
    ```
    *Note: if you have set a different port in the `.env` file obviously use that port instead.*
    It should return an `Ok` response.

## Setting up Cloudflare Tunnel

### Installing Cloudflared

The official and recommended process (which later on will allow us to do `apt upgrade` when new versions are released) is:

1. Create a `keyrings` folder if it does not already exist.
    
    ```bash
    sudo mkdir -p --mode=0755 /usr/share/keyrings
    ```
    
2. Install the public gpg key from the official repo:
    
    ```bash
    curl -fsSL https://pkg.cloudflare.com/cloudflare-main.gpg | sudo tee /usr/share/keyrings/cloudflare-main.gpg >/dev/null
    ```
    
3. Use the  key to sign the package:
    
    ```bash
    echo "deb [signed-by=/usr/share/keyrings/cloudflare-main.gpg] https://pkg.cloudflare.com/cloudflared any main" \
      | sudo tee /etc/apt/sources.list.d/cloudflared.list
    ```
    
4. Finally install the `cloudflared` package.
    
    ```bash
    sudo apt update && sudo apt install -y cloudflared
    ```
    
5. Check it with `cloudflared --version`.

### Configuring the tunnel

1. Go to [https://one.dash.cloudflare.com/](https://one.dash.cloudflare.com/) and log in with your Cloudflare account. The same account that manages the DNS records of the domain you intend to use.
2. Choose a team name, use your name for example, which will result in `johndoe.cloudflareaccess.com`.
3. Choose the Free Plan.
4. You'll need to put your credit card and address. They won't charge you but unfortunately it's a required step to use Cloudflare Tunnels. Once we are inside the Cloudflare Zero Trust dashboard, we can proceed.
5. Now, back on the terminal of our home server we do:
    
    ```bash
     cloudflared tunnel login
    ```
    
    And click the provided link.
    
6. In the page that will have opened we select our intended domain and choose 'Authorize'. We can now close the page.
7. We can now create the tunnel with `tunnel create` and giving it a name, choose the name you prefer. 
    
    ```bash
    cloudflared tunnel create my-home-server-tunnel
    ```
    
    Great, now let's write down (or copy) the ID associated to our tunnel. It will be something like `921f7213-14af-4bfc-bf32-3d35d10816e5` (example ID).
    
8. A credential's file will have been created inside our user directory, let's check it by doing `cd ~/.cloudflared` and using `ls` to print the files inside that directory. It should contain a `921f7213-14af-4bfc-bf32-3d35d10816e5.json` and a `cert.pem`. The first one are the tunnel credentials and the second one the certificate.
9. We'll create (in case it does not exist) a cloudflared directory inside `etc/` where we'll manage the actual cloudflared configuration. Let's do `sudo mkdir -p /etc/cloudflared` and then move our credentials file there:
    
    ```bash
    sudo mv ~/.cloudflared/921f7213-14af-4bfc-bf32-3d35d10816e5.json /etc/cloudflared/
    ```
    
    *Note: We are moving the credentials file but not the certificate because cloudflared tunnel needs to be global (not tied to our user) whereas the certificate (`cert.pem`) authenticates only our actual ubuntu user to the Cloudflare Zero Trust user, so it can stay inside `~/.cloudflared/cert.pem`.*
    
10. Now let's go to that folder (`cd /etc/cloudflared`), and create there a config file with `sudo micro config.yml`, put inside the file the following contents:
    
    ```yaml
    tunnel: 921f7213-14af-4bfc-bf32-3d35d10816e5
    credentials-file: /etc/cloudflared/921f7213-14af-4bfc-bf32-3d35d10816e5.json
    ingress:
      - hostname: typstapi.mydomain.com
        service: http://localhost:80  # Forward to Caddy via HTTP
      - service: http_status:404
    ```
    
    *Note: Obviously changing `921f7213-14af-4bfc-bf32-3d35d10816e5` with whatever ID your tunnel has* 
    *Also, if you already had a Cloudflare Tunnel set up and are just adding this service you only need to add the following lines:*
    ```
    - hostname: typstapi.mydomain.com
      service: http://localhost:80
    ```
    *between the `ingress:` line and the fallback (`- service: http_status:404`) line.*
    
    *Another note: We are using HTTP (instead of HTTPS) because Cloudflare already handles the TLS handshake, so there's no need for Caddy to also handle it.*
    

### Setting up the DNS records

1. Let's create the DNS route to the tunnel
    
    ```yaml
    cloudflared tunnel route dns my-home-server-tunnel typstapi.mydomain.com
    ```
    
    It should say that a CNAME record has been added to the domain which directs requests to that specific subdomain to the tunnel that goes to our home server on port 80.

    *Note: Cloudflare tunnels are encrypted and perfectly safe, your server's http port (`80`) is already open by default so there is no possible attack on this surface.*

    We can check the Cloudflare dashboard and see how a DNS record has been added to our domain.
        
2. Alright now the tunnel is properly configured to route connections to our subdomain to our host on port `80`, but we still need to create the service that handles these connections. Let's install the cloudflared service.
    
    ```bash
    sudo cloudflared service install
    ```
    
    It should print “*Linux service for cloudflared installed successfully”.*
    
3. Now let's start the service.
    
    ```bash
    sudo systemctl start cloudflared
    ```
    
    And check its status with `cloudflared tunnel info my-home-server-tunnel`.
    
    We can also check with `systemctl is-enabled cloudflared` if the service starts automatically in the case we reboot our home server. In case it is not enabled, we should enable it manually with:
    ```
    sudo systemctl enable cloudflared
    ```
    
    *Note: all this configuration of cloudflared and the dns records is kinda independent from docker and caddy, so if this is configured as following the instructions it is already properly set up and if later on we encounter problems, we know they are not related to the Cloudflare Tunnel setup.*

## Adjusting the `docker-compose.yml` to work along Caddy
Before setting up Caddy, we only need to slightly modify our current `docker-compose.yml` file. What we'll do is connect our typst-api container to a docker network which we'll call `typst-api-net`.

Leave the rest of the YML keys as they were and simply add:
```
services:
  typst-api:
    networks:
      - typst-api-net

networks:
  typst-api-net:
    name: typst-api-net
    external: true
```

<details>

<summary>Full docker-compose.yml example </summary>

```
services:
  typst-api:
    image: mapaor4/typst-api:latest
    ports:
      - "${PORT:-8080}:${PORT:-8080}"
    environment:
      - PORT=${PORT:-8080}
      - TYPST_FONT_PATHS=/fonts
      - MAX_PAYLOAD_SIZE=${MAX_PAYLOAD_SIZE:-52428800} # 50MB
      - COMPILATION_TIMEOUT=${COMPILATION_TIMEOUT:-10}
      - MAX_CONCURRENT_COMPILATIONS=${MAX_CONCURRENT_COMPILATIONS:-10}
      - AUTH_TOKEN=${AUTH_TOKEN:-}
      - CORS_ALLOWED_ORIGINS=${CORS_ALLOWED_ORIGINS:-}
    volumes:
      - ./fonts:/fonts:ro
      - typst-packages:/root/.cache/typst/packages
    networks:
      - typst-api-net
    restart: unless-stopped

volumes:
  typst-packages:

networks:
  typst-api-net:
    name: typst-api-net
    external: true
```

*Important note: This is just for you to clearly understand where to place the key-value pairs in the yaml file, but don't copy paste this compose example because it may be outdated. Always use the latest version published on GitHub: [https://github.com/Mapaor/typst-api/blob/main/docker-compose.yml](https://github.com/Mapaor/typst-api/blob/main/docker-compose.yml).*

</details>

Great, save the file. Now let's create the actual docker network (as we've declare it as an external network).

```bash
docker network create typst-api-net
```

And finally restart the container so that the new compose applies:

```bash
docker compose up -d --force-recreate
```

*Note: After recreating it, it will print something like ` ✔ Container typst-api-typst-api-1  Started`. This tells us the container's name (`typst-api-typst-api-1`), which is not the same as the compose service name (`typst-api`). For our next step (setting up Caddy) we will use the service name, both resolve correctly thanks to Docker's internal's DNS, but the container's name depends also on the folder's name where it was started so it's less reliable.

### Setting up Caddy

1. We are gonna use Caddy also as a container, so let's also create a directory for it, in my case I like to have it inside a `docker` folder inside my user directory:
    
    ```bash
    mkdir ~/docker/caddy
    cd docker/caddy
    ```
    
2. Now let's create here a `Caddyfile` (no extension, simply 'Caddyfile'), we can use `micro Caddyfile` and inside it, let's put the following:
    
    ```bash
    http://typst-api.mydomain.com {
      reverse_proxy typst-api:8080 {
        header_up X-Forwarded-Proto https
        header_up Host {host}
        header_up X-Real-IP {remote_host}
      }
    }
    ```
    *Remember to change `mydomain` with your actual domain and `8080` to whatever port you have set up in the `.env` file.

    *Note: You may see in Caddy's docs that they don't put 'http' before the domain, however leaving 'http' (instead of blank or 'https') is actually crucial for this to work as we are letting Cloudflare handle the TLS handshake.*
    
3. Now let's set up Caddy's using also a docker compose and using the latest image available. Let's do `micro docker-compose.yml` and put there:
    
    ```bash
    services:
      caddy:
        container_name: caddy
        image: caddy:latest
        restart: unless-stopped
        volumes:
          - ./Caddyfile:/etc/caddy/Caddyfile:ro
          - caddy_data:/data
          - caddy_config:/config
        ports:
          - "80:80"
          - "443:443"
        networks:
          - typst-api-net
          # Here we would put networks of other services in case we had more services
    
    volumes:
      caddy_data:
      caddy_config:
    
    networks:
      typst-api-net:
        external: true
      # And here would go more networks of other services as well 
    ```
    
Perfect now simply one thing left to do, run the caddy container:
```
docker compose up -d
```
*Note: if you had already run Caddy and have now modified the compose or the Caddyfile, use `docker compose up -d --force-recreate` instead.*

Wait a few seconds for it to be fully operative and then finally visit your subdomain `https://typstapi.mydomain.com`, it should print a json with the `typst-api` version and other useful info.

You can also use curl to ensure your API is accessible from your custom domain.
```
curl -X GET https://typstapi.mydomain.com/health
```
It should return an `Ok` response.

## Troubleshooting TIPs
In case you don't get it running on the first attempt, here are some useful commands to troubleshoot and narrow down what might be the actual problem:
- `docker inspect caddy --format='{{json .NetworkSettings.Networks}}'`
- `docker network inspect typst-api-net | grep Name`
- `docker ps`
- `docker network ls`
- `curl -vI https://typstapi.mydomain.com`
- `docker logs caddy`
- `docker logs typst-api-typst-api-1`
