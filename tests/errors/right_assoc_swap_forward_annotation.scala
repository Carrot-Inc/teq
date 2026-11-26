// An annotation of a right-associative extension's receiver cannot read the method's own parameter.
import scala.annotation.StaticAnnotation
class Mark(val i: Int) extends StaticAnnotation
object Lib:
  extension (@Mark(y) x: Int)
    def %:(y: Int): Int = x + y
@main def run(): Unit = println("ok")
// expect: 5:20: error: Not found: y
