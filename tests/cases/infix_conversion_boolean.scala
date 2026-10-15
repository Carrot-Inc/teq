// A receiver converted to `Boolean` keeps `&&`'s short circuit: the right operand is never evaluated.
import scala.language.implicitConversions
var log = ""
class C
implicit def convert(c: C): Boolean = { log += "v"; false }
def receiver(): C = { log += "r"; new C }
def arg(): Boolean = { log += "X"; true }
@main def run(): Unit =
  println(receiver() && arg())
  println(log)
