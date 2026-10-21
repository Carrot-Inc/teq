// expect: 5:7: error: unclosed quoted identifier
// An unterminated backquote ending a line is no operator the next line continues.
object Main {
  def bar =
    x `
      y
}
