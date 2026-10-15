// An overloaded `&&` member with a default argument: the receiver, the default, the call, the by-name uses.
var log = ""
class C:
  def &&(x: => Int, y: Int = {log += "d"; 4}): Int = {log += "m"; x + x + y}
  def &&(x: String): Int = -1
def recv(): C = { log += "r"; new C }
def arg(): Int = { log += "a"; 3 }
@main def run(): Unit = { println(recv() && arg()); println(log) }
