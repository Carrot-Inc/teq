// expect: 9:3: error: expected extension methods
// An extension's methods stand to the right of the lines around it: a def at the width of those lines, after a
// header whose last clause closes on a line of its own, is no method of the extension, which then has none.
// scalac: "Extension without extension methods" at the def's line.
object O:
  extension (
      s: String
  )
  def twice: String = s + s
