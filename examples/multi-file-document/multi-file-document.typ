#import "template.typ": * // Import your custom template

#show: template // Show it

#show emph: set text(rgb("#888888")) // Additional formatting

= This is a title, coming from the main document

=== Introduction

This text _also_ comes from the main document, because we are putting the introduction in the main document in our case. We can use `#lorem` here to express that the introduction might be long.

#lorem(70)


// Now let's include the chapters as Typst content
#include "chapters/chapter-1.typ"
#include "chapters/chapter-2.typ"

=== Epilogue

This is now again coming from the main document, it could be in a separate file called `epilogue.typ` but we chose to put it directly at the end of the main document. Also notice how the header still remains in this second page.
