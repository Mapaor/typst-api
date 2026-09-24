# How to use the examples
If you have cloned this repo, in your terminal, from the root directory of the repository you would run the following commands or HTTP requests.

(Changing the port `3000` to whatever port you have set in your `.env` file).

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

```bash
curl -X POST http://localhost:3000/compile -F "main=@examples/using-a-template/using-a-template.typ" --output "examples/using-a-template/using-a-template.pdf"
```

```bash
curl -X POST http://localhost:3000/compile -F "main=@examples/local-template/local-template.typ" -F "cv.typ=@examples/local-template/cv.typ" -F "utils.typ=@examples/local-template/utils.typ" -F "layouts/timeline.typ=@examples/local-template/layouts/timeline.typ" -F "layouts/prose.typ=@examples/local-template/layouts/prose.typ" -F "layouts/numbered-list.typ=@examples/local-template/layouts/numbered-list.typ" -F "layouts/header.typ=@examples/local-template/layouts/header.typ" -F "layouts/bullet-list.typ=@examples/local-template/layouts/bullet-list.typ" -F "example-cv.yml=@examples/local-template/example-cv.yml" --output "examples/local-template/local-template.pdf"
```

```bash
curl -X POST http://localhost:3000/compile -F "main=@examples/using-fonts/using-fonts.typ" -F "terminal-grotesque.ttf=@examples/using-fonts/terminal-grotesque.ttf" --output "examples/using-fonts/using-fonts.pdf"
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

```pwsh
Invoke-WebRequest -Uri http://localhost:3000/compile -Method Post -Form @{
    main = Get-Item "examples/using-a-template/using-a-template.typ"
} -OutFile "examples/using-a-template/using-a-template.pdf"
```

```pwsh
Invoke-WebRequest -Uri http://localhost:3000/compile -Method Post -Form @{
    "main" = Get-Item "examples/local-template/local-template.typ"
    "cv.typ" = Get-Item "examples/local-template/cv.typ"
    "utils.typ" = Get-Item "examples/local-template/utils.typ"
    "layouts/timeline.typ" = Get-Item "examples/local-template/layouts/timeline.typ"
    "layouts/prose.typ" = Get-Item "examples/local-template/layouts/prose.typ"
    "layouts/numbered-list.typ" = Get-Item "examples/local-template/layouts/numbered-list.typ"
    "layouts/header.typ" = Get-Item "examples/local-template/layouts/header.typ"
    "layouts/bullet-list.typ" = Get-Item "examples/local-template/layouts/bullet-list.typ"
    "example-cv.yml" = Get-Item "examples/local-template/example-cv.yml"
} -OutFile "examples/local-template/local-template.pdf"
```

```pwsh
Invoke-WebRequest -Uri http://localhost:3000/compile -Method Post -Form @{
    "main" = Get-Item "examples/using-fonts/using-fonts.typ"
    "terminal-grotesque.ttf" = Get-Item "examples/using-fonts/terminal-grotesque.ttf"
} -OutFile "examples/using-fonts/using-fonts.pdf"
```
