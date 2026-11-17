// expect: 11:10: error: object creation impossible, since def g: Int in trait Sh is not defined
// absent: 11:5: error: value += is not a member of Int
// expect: 1 error found
// An operand of an assignment operator whose only error the reference tells after its typer (an anonymous class that leaves a member undefined):
// the assignment form does not fail by it (the reference's typer tells nothing inside the try), so that
// error alone is told, and the operator's line of the first try is not.
trait Sh { def g: Int }
object T {
  def main(args: Array[String]): Unit = {
    var x: Int = 1
    x += new Sh { }.g
    println(x)
  }
}
