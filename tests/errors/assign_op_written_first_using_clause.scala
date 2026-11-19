// expect: 17:14: error: No given instance of type Token was found for parameter x$2 of method += in class E
// expect: 1 error found
// An assignment operator's application is typed as written first, and that try has failed only where an error
// was told before the operator's list was applied: a using clause of the operator that finds no instance is told
// after it, and the assignment form (`c(0) = c(0) + 1`, which would build here) is not tried. scalac tells this
// one line (E172).
trait Token
class E:
  def +=(v: Int)(using Token): Unit = println("member")
  def +(v: Int): E = { println("plus"); this }
class C:
  def apply(i: Int): E = { println("apply"); new E }
  def update(i: Int, v: E): Unit = println("update")
object Test:
  def main(args: Array[String]): Unit =
    val c = new C
    c(0) += 1
