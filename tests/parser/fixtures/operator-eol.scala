// An operator at a line's end whose operand is missing before the next definition.
object O:
  val a: Int = 1 +
  val b: Int = 2
  val bad: String = 3
