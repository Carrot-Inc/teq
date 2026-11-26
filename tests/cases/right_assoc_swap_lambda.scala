// A right-associative call whose left operand is a lambda: it is typed against the receiver's formal.
extension (f: Int => Int)
  def %:(n: Int): Int = f(n)
@main def run(): Unit = println((x => x + 1) %: 41)
