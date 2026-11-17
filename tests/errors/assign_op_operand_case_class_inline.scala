// expect: 14:49: error: Case class definitions are not allowed in inline methods or quoted code. Use a normal class instead.
// absent: 14:31: error: value += is not a member of Int
// expect: 1 error found
// An operand of an assignment operator whose only error the reference tells after its typer (a case class in an inline method's body):
// the assignment form does not fail by it (the reference's typer tells nothing inside the try), so that
// error alone is told, and the operator's line of the first try is not. PINNED, listed in tests/pending/errors.txt:
// the reference tells this error in its typer's phase, after the inline method's body is typed (PrepareInlineable),
// and teq tells it inside the try, so the operator's line is told beside it.
trait Sh { def g: Int }
class Base { def g: Int = 1; final def h: Int = 1 }
object T {
  var x: Int = 1
  val arr = Array(1, 2)
  inline def site(): Unit = { x += { case class P(a: Int); P(2).a } }
  def main(args: Array[String]): Unit = {
    site()
    println(x)
  }
}
