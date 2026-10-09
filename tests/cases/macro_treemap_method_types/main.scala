import Macros.*

final case class Box(n: Int)

def twice(b: Box): Int = b.n * 2

@main def run(): Unit =
  println(typed { val a = 1; val b = a + 1; b * 3 })
  println(typed { val s: Any = "x"; s != "y" })
  println(typed(List(1, 2).map(x => x * 2)))
  println(typed { val f = (x: Int) => x + 1; f(2) })
  println(typed(s"a${1 + 1}b"))
  println(typed(twice(Box(5))))
  println(typed { val ok = true; val no = false; (ok && no) || !no })
