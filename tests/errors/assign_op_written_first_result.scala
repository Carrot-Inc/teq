// expect: 15:19: error: Found:    String⏎Required: Int
// expect: 1 error found
// The first try of an assignment operator's application has failed only where an error was told before the
// operator's list was applied: its result's misfit with the expected type is told after it, and the assignment
// form (whose `update` gives an `Int` here) is not tried. scalac tells this one line (E007).
class E:
  def +=(n: Int): String = "member"
  def +(n: Int): E = { println("plus"); this }
class C:
  def apply(i: Int): E = new E
  def update(i: Int, v: E): Int = { println("update"); 42 }
object Test:
  def main(args: Array[String]): Unit =
    val c = new C
    val n: Int = (c(0) += 1)
    println(n)
