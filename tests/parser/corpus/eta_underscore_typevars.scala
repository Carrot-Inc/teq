def f(x: Int): Int = x + 1
def show(x: Any): String = x match
  case l: List[t] => s"list of ${l.size}"
  case m: Map[k, v] => s"map of ${m.size}"
  case _ => "other"
val _: Int = { println("init"); 3 }
object O:
  val _ = println("member init")
  val v = 1
@main def run(): Unit =
  val g = f _
  println(g(1))
  println(show(List(1, 2)))
  println(show(Map(1 -> 2)))
  println(O.v)
  val _: String = "x"
