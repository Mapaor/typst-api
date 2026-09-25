# Typst API

A high-performance, Dockerized REST API built in Rust to compile [Typst](https://typst.app/) documents into PDFs. It supports compiling documents with local assets, caching fonts for performance, and configurable resource limits.

## Spinning up the container

### Running with Docker
You can pull the pre-built image directly from Docker Hub:
```bash
docker run -p 8080:8080 -e mapaor4/typst-api:latest
```
The API will be available at `http://localhost:8080/compile`.

### Running with Docker Compose
1. Create a `.env` and add the variables you need for your case. See `.env.example` to understand the allowed variables.
2. Download de `docker-compose.yml` file from this repo and then do `docker compose up -d`.
3. The API will be available at `http://localhost:8080/compile` or whatever port you have specified.

### Running with Docker Compose (Local Development)

1. Clone this repo `git clone https://github.com/Mapaor/typst-api` (and `cd typst-api`).
2. Copy `.env.example` to `.env` and adjust the variables if needed.
3. No need to modify the `docker-compose.yml` because we have a `docker-compose.override.yml` that already does the job.
4. Run `docker compose up --build -d`. This will trigger the `Dockerfile` actions and start building the docker image and after that starting the container.
5. The API will be available at `http://localhost:8080/compile`.

## Using the API

### Source code simple request
Send an `application/json` request with your typst source code like shown in [`examples/typst-source-code/README.md`](./examples/typst-source-code/README.md).

```bash
curl -X POST http://localhost:8080/compile/source -H "Content-Type: application/json" -d '{"source": "= Hello Typst!\nThis is a very simple document.", "filename": "main.typ"}' --output "examples/typst-source-code/example.pdf"
```

You can also set CORS to `*` (permissive) in the env file and open the [`examples/typst-source-code/index.html`](examples/typst-source-code/index.html) file in your browser for a more interactive example.

### Multipart example Request

Send a `multipart/form-data` request with your Typst files, images and other files. The main file should either be explicitly named in the form field as `main` or named `main.typ` in its filename.

#### With curl
```bash
curl -X POST http://localhost:8080/compile \
  -F "main=@my_document.typ" \
  --output result.pdf
```
Note: If compilation fails, curl will save the JSON error response into result.pdf. To see the error in your console instead, remove the `--output` flag.

#### With powershell
```pwsh
try {
    Invoke-WebRequest -Uri http://localhost:8080/compile -Method Post -Form @{
        main = Get-Item "my_document.typ"
    } -OutFile "result.pdf" | Out-Null
    Write-Host "Success! Saved to result.pdf"
} catch {
    $_.ErrorDetails.Message
}
```

#### With Postman
If you have Postman desktop installed (and optionally the VSCode Extension as well), you can easily test the API by doing the following:
1. Create a new `POST` request to `http://localhost:8080/compile`.
2. Go to the Body tab and select `form-data` (instead of `raw`).
3. Add a key named `main`, change its type from `Text` to `File`.
4. Select (upload) your `.typ` file in the value column.
5. Click Send. Postman is great because it will show the visual PDF if the request is successful or it will show the JSON error if it fails.

#### More examples
You can check a full set of examples in the `examples/` directory, which contain both the Typst and related files, the PowerShell and Curl commands to run the examples and the expected PDF output.

## Exposing the API
You can then use any reverse proxy, tunnel or VPN you might typically use to expose your containers to your other devices or the whole internet.

My recommendations:
- If you are the only one who is gonna use the API from a specific set of devices, use Tailscale and make the requests to the tailnet IP of your server.
- If you want the API to be accessible through the internet, use Caddy as a reverse proxy on your server and then Cloudflare Tunnel (Cloudflared) for creating a tunnel between your host and the world.
Quick tip, use `http` instead of `https` or nothing (``) in the URL of the subdomain where you'll host your server in your Caddyfile.
    ```
    http://my.subdomain.com {
        reverse_proxy typst-api:8080 {
            header_up X-Forwarded-Proto https
            header_up Host {host}
            header_up X-Real-IP {remote_host}
        }
    }
    ```
- Use Pangolin (instead of Cloudflare Tunnel) running on a VPS if you plan to work with very big documents or concurrent requests and want to avoid the Cloudflare 100MB/s limitation.

Note: If you do expose the API running on your server to the general public (the internet) make sure to either enable token authentication (so that only you and people who you trust can  use the API) or enforce limits like maximum payload size and maximum concurrent compilation to prevent API usage.

## ROADMAP

- [X] Dynamic Custom Fonts: Implement support for users to upload custom `.ttf` or `.otf` font files as part of the `multipart/form-data` payload (allowing ephemeral custom fonts per compile request).
- [X] Allow to refresh the font cache without needing a container restart.
- [X] Add better diagnostics (return line and column) in the format_errors response.
- [X] Allow to make a HTTP request with typst code (instead of a typst file). Using an `application/json` new endpoint (which we'll call `/compile/source`).
- [X] Handle CORS with `tower-http` and add configuration options in the env file.
- [X] Add concurrency limits (not only timeout of individual requests but also a maximum of active compilations)
- [ ] Add other limits (maximum file count or maximum source size or maximum package fetching?) although maybe our current global payload limit already handles their combination correctly so that the API cannot be abused.
- [ ] Should we maybe cache the top 100 most used typst packages? or something similar.
- [X] Test the API authentication (token) manually.
- [X] Publish the first image of the library to DockerHub

#### After we have a first stable/complete version of the API
- [X]  Create an initial test suite
- [X]  Create an OpenAPI documentation

#### In the far future
- [ ] Implement something similar like a 'watch' option (like the CLI) for compiling a file that is constantly changing without having to compile it all again (only the parts that have changed). In other words, implement caching of compiled results. Typst already allows incremental compilation. We could maybe expose another layer of the API that works with websockets instead of http. Something like:
```
POST /sessions
  Creates a compilation session

WebSocket /sessions/{id}/watch
  Sends source changes and receives results

DELETE /sessions/{id}
  Releases server-side state
```
The server should use one actor/task per session so updates are serialized:
session actor
  -> receive update
  -> update virtual files
  -> compile
  -> render PDF/SVG
  -> send result
Without letting multiple concurrent requests mutate the same compilation state. The good thing is axum already supports websockets. We could also use socketioxide (like socket.io for rust). I don't know, I'll have to think about it.

Edit: Or maybe a 2-layer is not needed, a single WebSocket endpoint (ws://localhost:8080/watch) can be used, where the client connects, sends an initial "setup" payload with files, and then sends diffs or update messages. And when the client disconnects, the watch actor gets killed.

We could also do this all with HTTP maybe and reuse the session. Or we could somehow even provide a wasm of the typst compiler already initialized via http. I really don't know. Or maybe none of this is needed and debounce compilation on the client-side is already enough.

- [ ] Allow different outputs (PDF, SVG, PNG, HTML?). SVG can be generated with `typst-svg` and PNG probably from the SVG. Investigate how the Typst web app handles the (still experimental) HTML export.
- [ ] Allow output additional information (fomat eg. PDF or PDF-A, DPI, PDF metadata, etc.) Check the current output options of the typst compiler and the typst web app.
- [ ] Create a way to generate versioned docker images corresponding to a few typst compiler versions. Find a way to name them properly, for example: `typst-api:0.1.0-v0.15.1` or `typst-api:latest-v0.14.2`.
- [ ] Also create an endpoint to check the typst version and an endpoint to check a particular package version which typst requirements has. Maybe `typst-kit` already has some sort of package resolution/compatibility internal information(?).

## License
MIT