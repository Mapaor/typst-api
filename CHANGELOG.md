# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

[Unreleased]: https://github.com/mapaor4/typst-api/compare/v0.2.2...HEAD
[0.2.2]: https://github.com/mapaor4/typst-api/compare/v0.2.1...v0.2.2
[0.2.1]: https://github.com/mapaor4/typst-api/compare/v0.2.0...v0.2.1
[0.2.0]: https://github.com/mapaor4/typst-api/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/mapaor4/typst-api/releases/tag/v0.1.0