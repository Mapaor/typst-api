# How to use the examples
If you have cloned this repo, in your terminal, from the root directory of the repository you would run the following commands or HTTP requests.

(Changing the port '3000' to whatever you have set in your `.env` file).

## With curl
*Note: If compilation fails, curl will save the JSON error response into the `.pdf` file. To see the exact error in your console instead, remove the `--output` flag.*

```bash
curl -X POST http://localhost:3000/compile -F "main=@examples/simple/simple.typ" --output "examples/simple/simple.pdf"
```

```bash
curl -X POST http://localhost:3000/compile -F "main=@examples/math/math.typ" --output "examples/math/math.pdf"
```

```bash
curl -X POST http://localhost:3000/compile -F "main=@examples/using-images/using-images.typ" -F "images/cat.jpg=@examples/using-images/images/cat.jpg" -F "images/star.png=@examples/using-images/images/star.png" --output "examples/using-images/using-images.pdf"
```

```bash
curl -X POST http://localhost:3000/compile -F "main=@examples/multi-file-document/multi-file-document.typ" -F "template.typ=@examples/multi-file-document/template.typ" -F "chapters/chapter-1.typ=@examples/multi-file-document/chapters/chapter-1.typ" -F "chapters/chapter-2.typ=@examples/multi-file-document/chapters/chapter-2.typ" --output "examples/multi-file-document/multi-file-document.pdf"
```

```bash
curl -X POST http://localhost:3000/compile -F "main=@examples/using-packages/using-packages.typ" --output "examples/using-packages/using-packages.pdf"
```

## With powershell
*The perk of powershell here is that `Invoke-WebRequest` naturally handles errors, so if the compilation fails you will directly see the JSON with the error response.*

```pwsh
Invoke-WebRequest -Uri http://localhost:3000/compile -Method Post -Form @{
    main = Get-Item "examples/simple/simple.typ"
} -OutFile "examples/simple/simple.pdf"
```

```pwsh
Invoke-WebRequest -Uri http://localhost:3000/compile -Method Post -Form @{
    "main" = Get-Item "examples/using-images/using-images.typ"
    "images/cat.jpg" = Get-Item "examples/using-images/images/cat.jpg"
    "images/star.png" = Get-Item "examples/using-images/images/star.png"
} -OutFile "examples/using-images/using-images.pdf"
```

```pwsh
Invoke-WebRequest -Uri http://localhost:3000/compile -Method Post -Form @{
    "main" = Get-Item "examples/multi-file-document/multi-file-document.typ"
    "template.typ" = Get-Item "examples/multi-file-document/template.typ"
    "chapters/chapter-1.typ" = Get-Item "examples/multi-file-document/chapters/chapter-1.typ"
    "chapters/chapter-2.typ" = Get-Item "examples/multi-file-document/chapters/chapter-2.typ"
} -OutFile "examples/multi-file-document/multi-file-document.pdf"
```

```pwsh
Invoke-WebRequest -Uri http://localhost:3000/compile -Method Post -Form @{
    main = Get-Item "examples/using-packages/using-packages.typ"
} -OutFile "examples/using-packages/using-packages.pdf"
```