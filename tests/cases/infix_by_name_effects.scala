// A class's own `&&` and `||` with by-name parameters: receiver first, the argument per use.
var log = ""
class C:
  def &&(x: => Int): Int = { log += "m"; x + x }
  def ||(x: => Int): Int = { log += "n"; 7 }
def receiver(): C = { log += "r"; new C }
def arg(): Int = { log += "a"; 3 }
@main def run(): Unit =
  println(receiver() && arg())
  println(receiver() || arg())
  println(log)
