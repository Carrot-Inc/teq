// expect: 8:13: error: only defs are allowed in an extension
// A parameter clause after a blank line is no clause of the extension whose header stands before the blank line:
// the indented line is read as the extension's methods, and a `(` begins none. scalac: "end of extension method
// expected but '(' found" at the same place.
object O:
  extension (s: String)

            (using n: Int)
    def times: String = s * n
