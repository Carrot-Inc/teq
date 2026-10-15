// `&&` and `||` of Booleans evaluate their right operands only as needed.
var log = ""
def b(s: String, v: Boolean): Boolean = { log += s; v }
@main def run(): Unit =
  println(b("a", false) && b("X", true))
  println(b("b", true) || b("Y", false))
  println(b("c", true) && b("d", true))
  println(b("e", false) || b("f", true))
  println(log)
