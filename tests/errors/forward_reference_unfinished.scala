// expect: 9:21: error: type mismatch
// expect: 1 error found
// absent: forward reference
// A block's value whose initialiser does not type, read by a later method of the block that also names a method
// over a value: the type error alone, as scalac reports it (its forward-reference check does not run after a
// typer's error), and the value is no value under definition once its statement is left.
object Unfinished:
  def run(n: Int): Int =
    val held: Int = "text"
    def first: Int = held + second
    val more: Int = first
    def second: Int = n
    more
