#let template = doc => {
  set page(header: [
    #align(center)[#text(1.6em)[#smallcaps[
      This is a header, coming from the custom template
    ]]]
  ])
  doc
}
