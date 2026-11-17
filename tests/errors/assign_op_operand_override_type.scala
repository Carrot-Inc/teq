// expect: 14:23: error: error overriding method g in trait Sh of type Int: method g of type String has incompatible type
// absent: 14:5: error: value += is not a member of Int
// expect: 1 error found
// An operand of an assignment operator whose only error the reference tells after its typer (a member overriding one of another type):
// the assignment form does not fail by it (the reference's typer tells nothing inside the try), so that
// error alone is told, and the operator's line of the first try is not.
trait Sh { def g: Int }
class Base { def g: Int = 1; final def h: Int = 1 }
object T {
  var x: Int = 1
  val arr = Array(1, 2)

  def main(args: Array[String]): Unit = {
    x += new Sh { def g: String = "s" }.hashCode
    println(x)
  }
}
