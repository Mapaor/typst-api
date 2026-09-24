# How to use the examples
If you have cloned this repo, in your terminal, from the root directory of the repository you would run the following commands or HTTP requests.

(Changing the port '3000' to whatever you have set in your `.env` file).

## With curl
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