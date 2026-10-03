# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]
### Added
- Added new endpoint `POST /admin/fonts` to add new fonts to the database using `multipart/form-data`
- Created a static website `/add-fonts` that asks for `ADMIN_TOKEN` as input, and allows uploading font files. It internally calls `POST /admin/fonts`. The website is just for ease of use when adding new fonts to the font database.
### Changed
- Improved logs for when `CACHE_ALL_133_TYPST_FONTS` is set, added a timeout for each download and made the `FontState` update every 20 downloaded fonts.
- Improved the info returned for `GET /admin/info`.

## [0.2.6] - 2026-10-02
### Added
- Added env var `CACHE_ALL_133_TYPST_FONTS` to download, extract, and cache the 133 fonts from the Typst web app, from a curated mirror I created, which is hosted on [`Mapaor/typst-fonts-mirror`](https://github.com/Mapaor/typst-fonts-mirror).
- Also added the equivalent new endpoint `POST /admin/fonts/sync` to manually trigger a background sync of the font mirror repository. The idea is once again to run the server at startup with the env variable, and then use this endpoint to fetch updates.
- Added a new Docker named volume `typst-fonts` in `docker-compose.yml` to persist mirrored fonts.
- Fixed API hanging on startup if `CACHE_ALL_PACKAGES` was set.

## [0.2.5] - 2026-09-29
### Added
- Added installation scripts for bash and PowerShell and updated README with instructions.

### Changed
- Fixed playground styles and script linking by adding them inline in the HTML.

## [0.2.4] - 2026-09-29
### Added
- Added `/playground` route, a simple static website to test the API in the browser.
- Added `/admin/info` endpoint to check status, config values and stats.
- Added tower-http `trace` feature to improve logs.
- Added guides [`CUSTOM_DOMAIN_GUIDE.md`](CUSTOM_DOMAIN_GUIDE.md) and [`FUNNEL_GUIDE.md`](FUNNEL_GUIDE.md) for exposing the API publicly if you have a home server under CG-NAT.
- Added some more tests.

### Changed
- Now if `ADMIN_TOKEN` but `AUTH_TOKEN` isn't, `/compile` and `/compile/source` remain publicly accessible. This is better than everything under `ADMIN_TOKEN` (old logic). Now you can make your API public while still protecting the `admin/` routes.
- Fixed `docker-publish` workflow to properly update the release notes (internal detail).

## [0.2.3] - 2026-09-27
### Added
- Added a public root endpoint (`GET /`) that returns service metadata and links to the Swagger UI, OpenAPI specification, and health check.

## [0.2.2] - 2026-09-26
### Added
- Implemented `cargo-dist` to automatically generate and publish pre-compiled standalone binaries for Windows, macOS, and Linux across `x86_64` and `aarch64` architectures.
- Released binaries are now packaged with `assets/fonts` and a `.env.example` file.

## [0.2.1] - 2026-09-25
### Added
- Added ARM architecture support for the Docker image. The GitHub Actions workflow now builds and pushes both `linux/amd64` and `linux/arm64` images to Docker Hub.

## [0.2.0] - 2026-09-25
### Added
- **Package Cache Endpoints:**
  - `GET /admin/packages`: List cached packages and their total size.
  - `POST /admin/packages/preload`: Add a specific list of packages to the cache.
  - `POST /admin/packages/sync-all`: Download (or update) the entire Typst registry in the background.
  - `DELETE /admin/packages/cache`: Clear all downloaded packages.
- **Admin Authentication:** Added a new `ADMIN_TOKEN` environment variable to secure `/admin` endpoints. Falls back to `AUTH_TOKEN` if not set.
- **Configuration Options:**
  - `PRELOAD_PACKAGES`: Pre-downloads a specified list of packages on startup.
  - `CACHE_ALL_PACKAGES`: Downloads the entire Typst registry (~1.8GB) in the background on startup.
- Added a `typst-packages` Docker named volume in `docker-compose.yml` to persist the package cache across container restarts.

### Changed
- **Breaking:** Moved the `POST /fonts/refresh` endpoint to `POST /admin/fonts/refresh`.
- Refactored `main.rs` into separate files and moved the request handling logic into a clean `handlers/` directory.

## [0.1.0] - 2026-09-24
### Added
- Initial release of the Typst API, providing a dockerized REST API for compiling Typst documents into PDFs.
- Endpoint `POST /compile` to compile Typst files with multipart uploads for typst files, images, custom fonts, and other files (JSON/YAML).
- Endpoint `POST /compile/source` to compile inline Typst source code via JSON payload.
- Endpoint `GET /health` for server health checks.
- Endpoint `GET /fonts` to list available fonts.
- Endpoint `POST /fonts/refresh` to reload custom fonts from the mounted `fonts/` directory.
- Support for ephemeral fonts uploaded per request.
- Automatic on-demand downloading of Typst packages and templates from the Typst Universe.
- Complete compiler error messages and diagnostics returned in JSON format on failure.
- Optional Bearer-token authentication (`AUTH_TOKEN`).
- Configurable server port and API limits (max payload size, compilation timeout, max concurrent compilations).
- CORS configuration support.
- Example requests for different Typst documents using cURL and PowerShell.

[Unreleased]: https://github.com/Mapaor/typst-api/compare/v0.2.6...HEAD
[0.2.6]: https://github.com/Mapaor/typst-api/compare/v0.2.5...v0.2.6
[0.2.5]: https://github.com/Mapaor/typst-api/compare/v0.2.4...v0.2.5
[0.2.4]: https://github.com/Mapaor/typst-api/compare/v0.2.3...v0.2.4
[0.2.3]: https://github.com/Mapaor/typst-api/compare/v0.2.2...v0.2.3
[0.2.2]: https://github.com/Mapaor/typst-api/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/Mapaor/typst-api/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/Mapaor/typst-api/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/Mapaor/typst-api/releases/tag/v0.1.0