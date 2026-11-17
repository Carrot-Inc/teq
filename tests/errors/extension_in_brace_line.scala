// expect: 8:5: error: expected extension methods
// expect: 14:5: error: expected extension methods
// The lines in a brace have the width of the first line that follows the brace, so the def after an extension
// written on the brace's line is at that width and is no method of it; a line break right after the header's
// `(` gives the width to the parenthesis alone, and the brace still takes it from the def's line. scalac:
// "Extension without extension methods" at each def's line.
object OneLine { extension (x: Int)
    def next: Int = x + 1
}

object ParenEndsLine { extension (
    x: Int
  )
    def next: Int = x + 1
}
