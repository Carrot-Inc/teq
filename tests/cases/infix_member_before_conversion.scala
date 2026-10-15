// A class's own `&&` and `||` taken before a conversion of the receiver to `Boolean`.
import scala.language.implicitConversions
var log = ""
class C:
  def &&(x: => Boolean): Boolean = { log += "m"; x; true }
  def ||(x: => Boolean): Boolean = { log += "n"; x; false }
implicit def convert(c: C): Boolean = { log += "v"; false }
def receiver(): C = { log += "r"; new C }
def arg(): Boolean = { log += "a"; true }
@main def run(): Unit =
  println(receiver() && arg())
  println(receiver() || arg())
  println(log)
