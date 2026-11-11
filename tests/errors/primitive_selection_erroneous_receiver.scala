// expect: 12:15: error: Ambiguous overload. The overloaded alternatives of method bar1 in object Test with types
// expect: 13:16: error: Ambiguous overload. The overloaded alternatives of method bar2 in object Test with types
// expect: 14:15: error: Ambiguous overload. The overloaded alternatives of method bar1 in object Test with types
// expect: 3 errors found
// A primitive `==` selected on the erroneous receivers ambiguous overloads leave, twice with one key: the
// selection kept for a receiver is checked against the one made again under TEQ_CHECK_MEMOS.
object Test:
  def bar1(x: Any) = 1
  def bar1(x: String*) = 2
  def bar2[T](a: Class[? <: T]): Int = 1
  def bar2[T <: Number](a: Class[T], ints: Int*): Int = 2
  val first = bar1("") == 1
  val second = bar2(classOf[Integer]) == 1
  val third = bar1("") == 2
