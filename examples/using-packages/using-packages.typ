#import "@preview/cetz:0.5.2"
#import "@preview/mitex:0.2.7": *

#show math.equation: block.with(fill: white, inset: 1pt)

= Intro
Packages from the Typst Universe like Mitex or Cetz are not passed as separate files, they are simply imported. The API uses `SystemPackages` from `typst-kit` which fetches and caches packages "on-demand".

== Cetz example

==== Karl's picture:
// Create a new canvas to draw on
#cetz.canvas(length: 3cm, {
  import cetz.draw: *

  // Change the design for all elements after it
  set-style(
    // Design of arrow tips at the end of lines
    mark: (fill: black, scale: 2),
    // Design of lines
    stroke: (thickness: 0.4pt, cap: "round"),
    // Design of angles
    angle: (
      radius: 0.3,
      label-radius: .22,
      fill: green.lighten(80%),
      stroke: (paint: green.darken(50%)),
    ),
    // Design of all text elements with an anchor
    content: (padding: 1pt),
  )

  // Draws the grid behind the circle
  grid(
    (-1.5, -1.5),
    (1.4, 1.4),
    step: 0.5,
    stroke: gray + 0.2pt,
  )

  // Draw the unit circle
  circle((0, 0), radius: 1)

  // Draw the axis lines and axis labels
  line((-1.5, 0), (1.5, 0), mark: (end: "stealth"))
  content((), $ x $, anchor: "west")
  line((0, -1.5), (0, 1.5), mark: (end: "stealth"))
  content((), $ y $, anchor: "south")

  // Draw the number steps on the x-axis
  for (x, ct) in ((-1, $ -1 $), (-0.5, $ -1/2 $), (1, $ 1 $)) {
    line((x, 3pt), (x, -3pt))
    content((), anchor: "north", ct)
  }

  // Draw the number steps on the y-axis
  for (y, ct) in ((-1, $ -1 $), (-0.5, $ -1/2 $), (0.5, $ 1/2 $), (1, $ 1 $)) {
    line((3pt, y), (-3pt, y))
    content((), anchor: "east", ct)
  }

  // Draw the green angle
  cetz.angle.angle((0, 0), (1, 0), (1, calc.tan(30deg)), label: text(green, [#sym.alpha]))

  // Draw the hypothenuse of the triangle
  line((0, 0), (1, calc.tan(30deg)))

  // Change the stroke for all upcoming elements
  set-style(stroke: (thickness: 1.2pt))

  // Draw the inner opposite leg of the triangle:
  // "The intersection of a vertical line (|-) through (30deg, 1) and a horizontal line through (0, 0)"
  line((30deg, 1), ((), "|-", (0, 0)), stroke: (paint: red), name: "sin")
  // Place the text halfway through on the opposite leg
  content(("sin.start", 50%, "sin.end"), text(red)[$ sin alpha $])

  // Draw the adjacent leg of the triangle
  line("sin.end", (0, 0), stroke: (paint: blue), name: "cos")
  // Place the text halfway and position it below the line
  content(("cos.start", 50%, "cos.end"), text(blue)[$ cos alpha $], anchor: "north")

  // Draw the outer opposite leg of the triangle
  line((1, 0), (1, calc.tan(30deg)), name: "tan", stroke: (paint: orange))
  // Draw the tangent equasion at the top and to the right of the line
  content("tan.end", $ text(#orange, tan alpha) = text(#red, sin alpha) / text(#blue, cos alpha) $, anchor: "west")
})

==== Torus

#let draw-torus(
  fill: green,
  stroke: auto,
  outer-radius: 4,
  inner-radius: 1,
  theta-divisions: 100, // Steps around major circle.
  phi-divisions: 100, // Steps around minor circle.
  light-direction: (1, 1, 1), // Light source direction
  ambient-light: 0.2, // Ambient light intensity (0-1)
  diffuse-strength: 0.8, // Diffuse lighting strength (0-1)
) = {
  import calc: cos, max, min, pi, pow, sin, sqrt

  let get-torus-point(theta, phi) = {
    let x = (outer-radius + inner-radius * cos(phi)) * cos(theta)
    let y = (outer-radius + inner-radius * cos(phi)) * sin(theta)
    let z = inner-radius * sin(phi)
    return (x, y, z)
  }

  /// Calculate surface normal at given theta, phi.
  let get-torus-normal(theta, phi) = {
    let nx = cos(phi) * cos(theta)
    let ny = cos(phi) * sin(theta)
    let nz = sin(phi)
    (nx, ny, nz)
  }

  let normalize-vector(vec) = {
    let (x, y, z) = vec
    let length = sqrt(pow(x, 2) + pow(y, 2) + pow(z, 2))
    if length == 0 { return (0, 0, 0) }
    (x / length, y / length, z / length)
  }

  /// Calculate dot product of two vectors.
  let dot-product(v1, v2) = {
    let (x1, y1, z1) = v1
    let (x2, y2, z2) = v2
    x1 * x2 + y1 * y2 + z1 * z2
  }

  /// Calculate lighting intensity using Lambertian shading.
  let calculate-lighting(normal) = {
    let norm-light = normalize-vector(light-direction)
    let norm-normal = normalize-vector(normal)

    // Lambertian (diffuse) shading.
    let diffuse = max(0, dot-product(norm-normal, norm-light))

    // Combine ambient and diffuse lighting.
    let intensity = ambient-light + diffuse-strength * diffuse
    min(1, intensity) // Clamp to [0, 1].
  }

  /// Interpolate between two colors based on intensity
  let shade-color(color, intensity) = {
    // Convert intensity to RGB scaling.
    // Minimum brightness to avoid pure black.
    let scale = max(0.1, intensity)

    // For built-in colors, create a lighter/darker version
    if type(color) == std.color {
      return color.lighten(100% * (intensity - 0.5))
    }

    // For custom colors, you might need different handling
    color
  }

  for i in range(theta-divisions) {
    for j in range(phi-divisions) {
      let theta1 = (2 * pi * i) / theta-divisions
      let theta2 = (2 * pi * (i + 1)) / theta-divisions
      let phi1 = (2 * pi * j) / phi-divisions
      let phi2 = (2 * pi * (j + 1)) / phi-divisions

      let point1 = get-torus-point(theta1, phi1)
      let point2 = get-torus-point(theta2, phi1)
      let point3 = get-torus-point(theta2, phi2)
      let point4 = get-torus-point(theta1, phi2)

      // Calculate normal at the center of the rectangle for lighting
      let mid-theta = (theta1 + theta2) / 2
      let mid-phi = (phi1 + phi2) / 2
      let normal = get-torus-normal(mid-theta, mid-phi)

      // Calculate shading intensity
      let intensity = calculate-lighting(normal)

      // Apply shading to color
      let shaded-color = shade-color(fill, intensity)

      cetz.draw.line(
        point1,
        point2,
        point3,
        point4,
        close: true,
        fill: shaded-color,
        stroke: if stroke == auto { shaded-color } else { stroke },
      )
    }
  }
}

#cetz.canvas({
  import cetz.draw: *
  ortho(x: -70deg, y: 0deg, draw-torus(light-direction: (0, -1, 1)))
})



== Mitex  example

The Mitex package is super useful for using LaTeX code inside a Typst document.

#assert.eq(mitex-convert("\alpha x"), "alpha  x ")

Write inline equations like #mi("x") or #mi[y].

Also block equations (this example is from #text(blue.lighten(20%), link("https://katex.org/")[katex.org])):

#mitex(
  `
  \newcommand{\f}[2]{#1f(#2)}
  \f\relax{x} = \int_{-\infty}^\infty
    \f\hat\xi\,e^{2 \pi i \xi x}
    \,d\xi
`,
)

We also support text mode (in development):

#mitext(
  `
  \iftypst
    #set math.equation(numbering: "(1)", supplement: "equation")
  \fi

  \subsection{Title}

  A \textbf{strong} text, a \emph{emph} text and inline equation $x + y$.

  Also block \eqref{eq:pythagoras}.

  \begin{equation}
    a^2 + b^2 = c^2 \label{eq:pythagoras}
  \end{equation}
`,
)

And everything you can imagine...

= More examples!

#mitex(
  `
\lim_{x \to \infty}a_n\qquad
\frac{d}{dt}\qquad
\ddot{x}\qquad
\frac{\partial^2}{\partial x^2}\qquad
\sum_{i=1}^na_i\qquad
\int_a^bf(t)dt\qquad
\oint_C \vec{F} \cdot \overrightarrow{d r}
`,
)

#mitex(
  `
\underbrace{1-e^0}_{0}+6
\qquad\qquad
\overbrace{
\frac{\partial f}{\partial x}
\dot{x}+\frac{\partial f}{\partial y}\dot{y}
+\frac{\partial f}{\partial t}
}^{\frac{df}{dt}}=0
\qquad\quad
\underbrace{a+a+a}_{
\substack{
\text{1st line of text} \\
\text{2nd line of text}
}
}
`,
)


#mitex(
  `
\begin{pmatrix}
a_{11}&a_{12}&a_{13}\\
a_{21}&a_{22}&a_{23}\\
a_{31}&a_{32}&a_{33}
\end{pmatrix}

\qquad \quad

\begin{bmatrix}
v_1\\
v_2\\
v_3
\end{bmatrix}

\qquad \quad

\begin{vmatrix}
\textbf{i}&\textbf{j}&\textbf{k}\\
v_1&v_2&v_3\\
u_1&u_2&u_3
\end{vmatrix}



`,
)

#mitex(
  `
f(x,y)=
\begin{cases}
\frac{x \sin (x^2+y^2)}{x^2+y^2} & \text {if}(x,y)\neq(0,0)
\\
0 & \text {if}(x,y)=(0,0)
\end{cases}

`,
)
