// expect: 14:34: error: method h cannot override final member method h in class Base
// absent: 14:5: error: value += is not a member of Int
// expect: 1 error found
// An operand of an assignment operator whose only error the reference tells after its typer (a final member overridden):
// the assignment form does not fail by it (the reference's typer tells nothing inside the try), so that
// error alone is told, and the operator's line of the first try is not.
trait Sh { def g: Int }
class Base { def g: Int = 1; final def h: Int = 1 }
object T {
  var x: Int = 1
  val arr = Array(1, 2)

  def main(args: Array[String]): Unit = {
    x += new Base { override def h = 2 }.h
    println(x)
  }
}
