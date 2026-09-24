Put your custom fonts in this directory.

If you add new fonts while your container is running refresh the cache by doing a POST request to '/fonts/refresh/'.

Alternatevly you can simply restart your container.

This directory is the default one, but you can use any other one by changing the TYPST_FONT_PATHS variable in the '.env' file.