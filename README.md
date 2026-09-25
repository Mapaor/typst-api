# Typst API

A high-performance, Dockerized REST API built in Rust to compile [Typst](https://typst.app/) documents into PDFs. It supports compiling documents with local assets, caching fonts for performance, and configurable resource limits.

The recommended way to run the server is with Docker Compose and an environment file. You can also run a standalone binary from the GitHub releases, or compile and run the source code with Cargo. The intended use of this repository is to let people self-host a simple Typst server without complications.

## Requirements
For most documents you only need about 35MB of RAM, however for a very large textbook with images, custom fonts, etc. you might require about ~200MB.

The requirements for most cases are:
- 0.5 GB of RAM
- 1 GB of disk space
- 1 core of CPU/vCPU

For cases where you are exposing your API to the public you should be try to have:
- 2 GB of RAM
- 3 GB of disk space
- 1 core of CPU/vCPU

Note: if you are exposing your API to the public be sure to prevent API abuse by configuring maximum payload size and maximum concurrent compilations as explained below.

## Starting the server

### Running with Docker Compose
1. In a directory of your choice on your server create 2 files, a `.env` and a `docker-compose.yml`, and create a `fonts` directory.
2. Add to the `.env` the contents of [`.env.example`](.env.example), configure the variables to your needs.
3. Put the contents of the [`docker-compose.yml`](docker-compose.yml) into your local docker compose file. It already has the proper configuration and pulls the Docker image from DockerHub.
4. From this same local directory, do `docker compose up -d`.
5. The API will be available at `http://localhost:8080/compile`, or replacing `8080` with whatever port you have specified.

### Running with Docker Compose (Local Development)

1. Clone this repo `git clone https://github.com/Mapaor/typst-api` (and do `cd typst-api`).
2. Copy the `.env.example` to `.env` and adjust the variables if needed.
3. No need to modify the `docker-compose.yml` because we have a `docker-compose.override.yml` that already does the job.
4. Run `docker compose up --build -d`. This will trigger the `Dockerfile` actions and start building the docker image and after that starting the container.
5. The API will be available at `http://localhost:8080/compile`.
6. Now you can modify the Rust source code as you please and every time you hit a `docker compose up --build -d` the container will restart with your new custom image. This way you can tweak the API even further to fit your exact needs.

### Running with Docker
You can pull and run the image directly from Docker Hub by using:
```bash
docker run -p 8080:8080 mapaor4/typst-api:latest
```
The API will be available at `http://localhost:8080/compile`. You can of course set any another port you like. For example, you can map the container's default port (`8080`) to the host's `3001` port:

```bash
docker run -p 3001:8080 mapaor4/typst-api:latest
```
The API will now be available at `http://localhost:3001/compile`. 

### Running the standalone compiled binary

Download the binary from [GitHub releases](https://github.com/Mapaor/typst-api/releases), then execute it.

The server listens on `http://localhost:8080` by default. If you want to set some environment variables, set them before starting it.

On bash (Linux and macOS):

```bash
PORT=3001 ./typst-api
```

On PowerShell:

```powershell
$env:PORT = "3001"
.\typst-api.exe
```

If you prefer to use a `.env` file, put it in the same directory as the binary and (important or it won't work) also run the binary from that same directory. See `.env.example` for the complete list of allowed variables.

### Compiling from source (Local Development)

The API is a Rust binary crate, so if you have Rust installed you can clone the repo and run it with:

```bash
cargo run --release
```

You can run it with env variables as we've seen before.

On bash:

```bash
PORT=3001 cargo run --release
```

On PowerShell:

```powershell
$env:PORT = "3001"
cargo run --release
```

Or you can simply create a `.env` file (or copy and rename the `.env.example`) in the root of the repository and set the variables there, it will be loaded automatically.

You can also use `cargo build --release` to build the binary without running it, and then run the binary located in `target/release/typst-api` like if it were a pre-compiled binary.

You could do `cargo run` or `cargo build` without the `--release` flag for local development but for production (the actual API usage) the flag is recommended (it does additional performance optimizations for the final binary whereas without the flag these are avoided to reduce the build time).

### Why there is no crates.io package

`typst-api` is an application binary rather than a Rust library you can reuse. The Docker image lives on Docker Hub and the standalone binaries on GitHub. If you want you can still compile it from source using Cargo. The crate has not been published to crates.io on the one hand to state that the recommended way to host the API is with Docker, and on the other hand to avoid any potential confusion with the official Typst crates such as `typst-pdf`, `typst-cli`, `typst-kit`, etc.

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

### Admin Endpoints
The API includes admin endpoints which are prefixed with `/admin`. To set them add an `ADMIN_TOKEN` to the `.env` file. If the variable is no set these endpoints fall back to `AUTH_TOKEN` authentication. If neither is set, all endpoints are they are publicly accessible.

- `POST /admin/fonts/refresh` reloads local fonts (from the `TYPST_FONT_PATHS` directory) without having to restart the container.
- `GET /admin/packages` returns a list of all cached Typst packages and their total size.
- `POST /admin/packages/preload` receives a JSON array of packages to be downloaded and added to the cache.
- `POST /admin/packages/sync-all` triggers a background sync of all packages from the [Typst registry](https://packages.typst.org/preview/index.json). As of september 2026 all the  versions of all the packages are about 1.8 GB. 
- `DELETE /admin/packages/cache` clears the local package cache.

By default, when using Docker Compose the package cache persists container restarts because it uses a docker named volume, additionaly you can configure in the `.env` file the following variables:
- `PRELOAD_PACKAGES`: A list of comma-separated essential packages you want to have available on start up.
- `CACHE_ALL_PACKAGES`: If set to true downloads the entire Typst registry (~1.8GB) in the background when the server starts.

## Exposing the API
You can then use any reverse proxy, tunnel or VPN you might typically use to expose your containers to your other devices or the whole internet.

My recommendations:
- If you are the only one who is gonna use the API from a specific set owned devices, use Tailscale and make the requests to the tailnet IP of your server or home server.
- If you have a home server (without a public IP) and you want the API to be accessible through the internet, use Caddy as a reverse proxy on your server and then Cloudflare Tunnel (Cloudflared) for creating a tunnel between your host and the world, you'll need a domain of course.
  -> Checkout my the guide to set it up with Caddy and Cloudflare Tunnel on a Ubuntu Server.
- Use Pangolin (instead of Cloudflare Tunnel) running on a VPS if you plan to work with very big documents or concurrent requests and want to avoid the Cloudflare 100MB limitation.

Note: If you do expose the API running on your server to the general public (the internet) make sure to either enable token authentication (so that only you and people who you trust can  use the API) or enforce limits like maximum payload size and maximum concurrent compilation to prevent API usage.

## ROADMAP

- [X] Dynamic Custom Fonts: Implement support for users to upload custom `.ttf` or `.otf` font files as part of the `multipart/form-data` payload (allowing ephemeral custom fonts per compile request).
- [X] Allow to refresh the font cache without needing a container restart.
- [X] Add better diagnostics (return line and column) in the format_errors response.
- [X] Allow to make a HTTP request with typst code (instead of a typst file). Using an `application/json` new endpoint (which we'll call `/compile/source`).
- [X] Handle CORS with `tower-http` and add configuration options in the env file.
- [X] Add concurrency limits (not only timeout of individual requests but also a maximum of active compilations)
- [X] Test the API authentication (token) manually.
- [X] Publish the first image of the library to DockerHub
- [X] Handle package cache properly, create config options in the env file as well as endpoints for handling them.

#### After we have a first stable/complete version of the API
- [X] Create an initial test suite
- [X] Create an OpenAPI documentation
- [X] Generate also a `linux/arm64` docker image (besides the current `linux/amd64`).
- [X] Handle the creation and publishing of binaries for new releases with `cargo-dist`

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
- [ ] Add other limits (maximum file count or maximum source size or maximum package fetching?) although maybe our current global payload limit already handles their combination correctly so that the API cannot be abused. No needed for now, if someone imports lots of packages they'll hit the timeout limit.

## License
[MIT](LICENSE)