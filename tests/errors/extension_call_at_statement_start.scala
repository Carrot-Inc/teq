// expect: 9:15: error: expected an identifier, found literal
// expect: 14:16: error: expected ':', found ')'
// The word `extension` at a statement's start with a `(` after it begins an extension, a method of that name in
// scope or none: its arguments are read as a parameter clause. scalac: "an identifier expected, but integer
// literal found" and "':' expected, but ')' found" at the same places.
object A:
  def extension(x: Int): Int = x + 1
  val r: Int =
    extension(1)
object B:
  def extension(x: Int): Int = x + 1
  def r: Int =
    val y = 1
    extension(y)
