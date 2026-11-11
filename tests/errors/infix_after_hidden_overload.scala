// teq: --source future
// scalac 3.8.4 hides the erroneous overload message, which takes no position.
class C:
  def f(x: Int): Int = 1
  def f(x: String): Missing = ???
object Test:
  val use = new C f true
// expect: 5:21: error: type Missing not found
// expect: 7:19: error: Alphanumeric method f is not declared infix; it should not be used as infix operator.
// expect: 2 errors found
// absent: None of the overloaded alternatives
