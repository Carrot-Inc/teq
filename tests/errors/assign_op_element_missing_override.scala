// expect: 14:30: error: method g needs `override` modifier to override method g in class Base
// absent: 14:5: error: value += is not a member of Int
// expect: 1 error found
// An operand of an assignment operator whose only error the reference tells after its typer (a member without its `override`, on an element):
// the assignment form does not fail by it (the reference's typer tells nothing inside the try), so that
// error alone is told, and the operator's line of the first try is not.
trait Sh { def g: Int }
class Base { def g: Int = 1; final def h: Int = 1 }
object T {
  var x: Int = 1
  val arr = Array(1, 2)

  def main(args: Array[String]): Unit = {
    arr(0) += new Base { def g = 2 }.g
    println(x)
  }
}
