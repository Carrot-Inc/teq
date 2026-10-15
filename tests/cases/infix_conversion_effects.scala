// `&&` reached through an implicit conversion of the receiver, its by-name argument evaluated per use.
import scala.language.implicitConversions
var log = ""
class C
class D:
  def &&(x: => Int): Int = { log += "m"; x + x }
implicit def convert(c: C): D = { log += "v"; new D }
def receiver(): C = { log += "r"; new C }
def arg(): Int = { log += "a"; 3 }
@main def run(): Unit =
  println(receiver() && arg())
  println(log)
