#set text(font: "IBM Plex Sans", size: 11pt)
= How to use fonts in `typst-api`

There are 4 ways in which you can use fonts in this library.

=== 1. Using the default fonts
The docker image comes with a set of default fonts that you can use without any additional configuration. These are:
#set text(font: "New Computer Modern")
- New Computer Modern
- $"New Computer Modern Math"$
#set text(font: "Libertinus Serif")
- Libertinus Serif
- `Dejavu Sans Mono`
#set text(font: "Inter")
- Inter
#set text(font: "IBM Plex Sans")
- IBM Plex Sans

The first 4 are the default fonts used in the `typst` library, while the last 2 are additional fonts I've added because I really like them and I think it's nice to have at least one built-in font that looks nice or modern.

#set text(font: "New Computer Modern")
The default font for regular text is 'New Computer Modern', which looks like the text you are seeing right now. #lorem(40).

For using a custom font, you just need to specify the font name in your document. Like this: `#set text(font: "Inter")`.

#set text(font: "Inter", style: "normal", size: 11pt)
Notice how the font has changed from the default 'New Computer Modern' to 'Inter'. Let's add some dummy text to see how it looks: #lorem(40).

=== 2. Using system fonts
Because a containerized environment does not have access to the host fonts, if you want your container to access your server system's fonts, you must mount your font directory into the container's `/usr/share/fonts` directory (which is where the API automatically searches for system fonts).

You can do this by adding a volume to your `docker-compose.yml`:

For Linux:

```yaml
volumes:
  - /usr/share/fonts:/usr/share/fonts/host:ro
```

For Windows:
```yaml
volumes:
  - C:\Windows\Fonts:/usr/share/fonts/windows:ro
```

For macOS:
```yaml
volumes:
  - /Library/Fonts:/usr/share/fonts/mac:ro
```

Note: If instead of docker you are running the API natively using `cargo run`, your system's fonts will be detected and loaded automatically.

=== 3. Using a custom `fonts/` directory (recommended)
This API is designed so that you can put a set of `.ttf`, `.otf` or `.ttc` files in a `fonts/` directory next to your `docker-compose.yml` and `.env` files. If you do so, all these fonts will be available by default to the typst compiler.

_Note: You can customize the name of the directory where you want to store your custom fonts in the `.env` file._

A great thing implemented in this API, is that if you add new fonts to the `fonts/` directory while the container is running, you don't need to restart it! You can just send a POST request to `/fonts/refresh` to hot-reload the font cache. And you can always send a GET request to `/fonts` to see all available fonts.

=== 4. Dynamic custom fonts, passing the font files in the HTTP request (not recommended)
If you really need to, you can also pass fonts in the HTTP request. This is not recommended because it will make your request much larger and slower, but it is possible.

Just add your `.ttf`, `.otf`, or `.ttc` files as part of the `multipart/form-data` payload when hitting the `/compile` endpoint. The API will load these fonts ephemerally just for that specific compilation, without polluting the global font cache.

==== Let's use a custom font in our example
#set text(font: "Terminal Grotesque")
This paragraph is supposed to be displayed in the font Terminal Grotesque. The font file (`terminal-grotesque.ttf`) is on this same example folder that contains this `using-fonts.typ` Typst document and the corresponding PDF. If you run this example passing the font file in the request as part of the `multipart/form-data` you'll see this text with the proper font.

In powershell that would look like this:

```pwsh
Invoke-WebRequest -Uri http://localhost:3000/compile -Method Post -Form @{
    "main" = Get-Item "examples/using-fonts/using-fonts.typ"
    "terminal-grotesque.ttf" = Get-Item "examples/using-fonts/terminal-grotesque.ttf"
} -OutFile "examples/using-fonts/using-fonts.pdf"
```

You can obviously also put the `terminal-grotesque.ttf` file in your `fonts/` directory, reload the container or refresh the cache (POST `/fonts/refresh`) and the text will also appear in the proper font without you needing to pass the font in the request, therefore making the request much faster.
