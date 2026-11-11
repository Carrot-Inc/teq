import scala.language.implicitConversions

class Wrapper { def f: Int = 2 }
class Num
object Num { extension (x: Num) def f = 1 }
object Conversions { given Conversion[Num, Wrapper] = _ => new Wrapper }
import Conversions.given

@main def run(): Unit =
  val result: Int = (new Num).f
  println(result)
