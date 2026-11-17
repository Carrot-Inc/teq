// expect: 22:11: error: no given instance of type Tok was found for parameter t
// expect: 24:14: error: no given instance of type Tok was found for parameter t
// expect: 25:16: error: no given instance of type Tok was found for parameter t
// expect: 3 errors found
// The first try of an assignment operator's application has not failed where a member that does not take the
// argument gave way to an extension that does: the extension's using clause, which finds no instance, is told
// after its list was applied, and the assignment form (which would build at each line) is not tried; a tuple
// operand the extension takes whole alike. scalac tells these three lines (E172).
trait Tok
class E(val n: Int):
  def +=(s: String): Unit = println("member")
  def +(i: Int): E = { println("plus"); new E(n + i) }
  def +(p: (Int, Int)): E = { println("plus"); new E(n + p._1 + p._2) }
extension (e: E) def +=(i: Int)(using t: Tok): Unit = println("ext")
extension (e: E) def +=(p: (Int, Int))(using t: Tok): Unit = println("ext")
class C:
  def apply(i: Int): E = new E(0)
  def update(i: Int, e: E): Unit = println("update")
object Test:
  def main(args: Array[String]): Unit =
    var e = new E(0)
    e += 1
    val c = new C
    c(0) += 1
    e += (1, 2)
