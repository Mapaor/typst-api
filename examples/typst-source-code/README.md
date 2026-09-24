# Using Typst code instead of files
If you just want to send a simple Typst code without additional files, images or fonts, you can use the endpoint `/compile/source` which works as `application/json` (instead of `multipart/form-data`).

Here is a simple example.

### With curl
```bash
curl -X POST http://localhost:3000/compile/source -H "Content-Type: application/json" -d '{"source": "= Hello Typst\n\nThis is an example to test the `/compile/source` endpoint of the API. No Typst files were generated nor needed to create this PDF, only the Typst source code as a string.", "filename": "main.typ"}' --output "examples/typst-source-code/example.pdf"
```

### With powershell
In PowerShell `ConvertTo-Jsonp` already handles the proper escaping of the source string.
```pwsh
$body = @{
    source = "= Hello Typst`n`nThis is an example to test the ``/compile/source`` endpoint of the API. No Typst files were generated nor needed to create this PDF, only the Typst source code as a string."
    filename = "main.typ"
} | ConvertTo-Json

Invoke-WebRequest -Uri http://localhost:3000/compile/source `
  -Method Post `
  -Body $body `
  -ContentType "application/json" `
  -OutFile "examples/typst-source-code/example.pdf" | Out-Null
```

### With javascript
```javascript
const fs = require("node:fs/promises");

const response = await fetch("http://localhost:3000/compile/source", {
  method: "POST",
  headers: { "Content-Type": "application/json" },
  body: JSON.stringify({
    source:
      "= Hello Typst\n\nThis is an example to test the `/compile/source` endpoint of the API. No Typst files were generated nor needed to create this PDF, only the Typst source code as a string.",
    filename: "main.typ",
  }),
});

if (!response.ok) {
  throw new Error(`${response.status} ${await response.text()}`);
}

await fs.writeFile("examples/typst-source-code/example.pdf", Buffer.from(await response.arrayBuffer()));
```

### With python
```python
import json
from urllib.error import HTTPError
from urllib.request import Request, urlopen

payload = {
    "source": "= Hello Typst\n\nThis is an example to test the `/compile/source` endpoint of the API. No Typst files were generated nor needed to create this PDF, only the Typst source code as a string.",
    "filename": "main.typ",
}
request = Request(
    "http://localhost:3000/compile/source",
    data=json.dumps(payload).encode("utf-8"),
    headers={"Content-Type": "application/json"},
    method="POST",
)

try:
    with urlopen(request) as response:
        pdf = response.read()
except HTTPError as error:
    raise RuntimeError(error.read().decode("utf-8")) from error

with open("examples/typst-source-code/example.pdf", "wb") as output:
    output.write(pdf)
```