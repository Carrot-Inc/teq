package fix.interparts

// Interpolation parts as scalac pickles them, against the capture's (tests/tasty.sh): as
// written, but for `$$` and unicode escapes; an `s` interpolation's other escapes processed where
// it runs.
object InterpParts:
  def newline(x: Int): String = s"\n$x"
  def backslash(x: Int): String = s"\\n$x"
  def rawNewline(x: Int): String = raw"\n$x"
  def dollar(x: Int): String = s"a$$b$x"
  def unicode(x: Int): String = s"A$x\t"
  def quoted(x: Int): String = s"""a\n"q"$x"""
  def empty(x: Int, y: Int): String = s"$x$y"
