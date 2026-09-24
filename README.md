# Typst API

A high-performance, Dockerized REST API built in Rust to compile [Typst](https://typst.app/) documents into PDFs. It supports compiling documents with local assets, caching fonts for performance, and configurable resource limits.

## Usage

### Running with Docker Compose

1. Copy `.env.example` to `.env` and adjust the variables if needed.
2. Run `docker compose up -d`.
3. The API will be available at `http://localhost:8080/compile`.

### Example Request

Send a `multipart/form-data` request with your Typst files. The main file should either be explicitly named in the form field as `main` or named `main.typ` in its filename.

#### With curl
```bash
curl -X POST http://localhost:8080/compile \
  -F "main=@my_document.typ" \
  -F "logo.png=@assets/logo.png" \
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
If you have Postman desktop installed (and optionally the VSCode Extension as well), you can easily test the API:
1. Create a new `POST` request to `http://localhost:8080/compile`.
2. Go to the **Body** tab and select `form-data`.
3. Add a key named `main` (change its type from `Text` to `File` by hovering over the key field).
4. Select your `.typ` file in the value column.
5. Click **Send**! Postman will show the visual PDF if successful, or gracefully show the JSON error if it fails.

## To-Do List

- [ ] **Dynamic Custom Fonts via Multipart:** Implement support for users to upload custom `.ttf` or `.otf` font files as part of the `multipart/form-data` payload on a per-request basis (allowing ephemeral custom fonts per compile).
- [ ] **User-Defined Custom Fonts:** Further improve the global custom font loading logic (currently loaded via the mounted `/fonts` directory on startup) to allow dynamically refreshing the font cache or hot-reloading user fonts without needing a container restart.
