# Typst API Architecture

A brief outline on how this library works on a technical level.

## HTTP Requests
This library is an HTTP REST API, it relies mainly on the `axum` crate for handling requests asynchronously. In the case of files they are `multipart/form-data` requests, in the case of inline code they are `application/json` requests.

## Shared state
Shared state is managed via atomic reference counting (`Arc` from the Rust standard library). This allows multiple concurrent request handlers to share data (like configuration and cached packages) without copying it. For mutable state, like the global font database, it is wrapped in an asynchronous read-write lock from Tokio (`tokio::sync::RwLock`). This allows many simultaneous readers (concurrent compilations) but still preserves exclusive access when fonts are reloaded.

## File handling
Each request compiles in an isolated environment where files are parsed and handled as an in-memory virtual filesystem.

On a Typst level, the `World` trait is used, which acts as an interface between the compiler and the external world (fonts, files, images...). We implement a custom struct `ApiWorld` that works as an interface for this trait.

## Fonts
The server loads global fonts via a dedicated font database at startup using the `fontdb` crate. This database can be updated dynamically via a specific admin endpoint (`/admin/fonts/refresh`).

Requests may also include font files which are treated as ephemeral and never cached, for each request a merged (global and ephemeral) font book is created before compilation.

## Typst packages
The application uses `SystemPackages` from the `typst-kit` crate to resolve external Typst package dependencies. It downloads packages on demand and caches them globally.

Also the server provides admin endpoints (`/admin/packages/...`) to manage this cache.

## Execution and Safety
Typst compilations are CPU-bound. To prevent API abuse, a blocking thread pool is used, allowing to set maximum payload limits or timeouts, so that the primary asynchronous worker never gets  blocked. 

The API can also be made private or secure using optional bearer tokens. The middleware can protect the compilation endpoints under the `AUTH_TOKEN` and package and font cache manipulation under the `ADMIN_TOKEN`. If the latter is not set those endpoints default to the `AUTH_TOKEN`, if none is set all endpoints are publicly available.

## Error messages
One of the great advantages of Typst (over LaTeX) is the useful diagnostics it provides when a compilation error or warning happens. In our API we extract these compiler error messages and span identifiers and transform them into a JSON response.
