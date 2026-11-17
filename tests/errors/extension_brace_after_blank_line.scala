// expect: 7:3: error: expected extension methods
// A brace after a blank line is no part of the extension whose header stands before the blank line: the
// extension has no methods. scalac: "Extension without extension methods" at the brace's line.
object O:
  extension (s: String)

  {
    def twice: String = s + s
  }
