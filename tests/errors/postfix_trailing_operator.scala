// A postfix operator at the end of a line does not take the next line as its operand: refused.
// expect: 7:7: error: recursive value x needs type
import scala.language.postfixOps
class C { def unary_! : Int = 8; def !! : Int = 4 }
@main def run(): Unit =
  val c = new C
  val x = c !!
  println(x)
