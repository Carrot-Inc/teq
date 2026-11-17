import lib.*
class K2(override val v: String) extends K1
def show(t: T1): String = t.v
@main def run(): Unit =
  println(new K1().other)
  println(show(new K2("k2-arg")))
