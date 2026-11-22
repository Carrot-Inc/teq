// An enum read from its source: its constants, values() and valueOf, a constant's arguments and body (javac's), and a
// match over its constants.
object Main {
  def f(e: E): Int = e match { case E.A => 1; case E.B => 2; case E.C => 3 }
  def main(args: Array[String]): Unit = {
    println(E.B.ordinal + " " + E.values().length + " " + E.valueOf("C").name)
    println(E.B.w() + " " + E.C.w() + " " + f(E.C))
  }
}
