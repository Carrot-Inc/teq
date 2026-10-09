def f(x: Any): String = x match
  case _: (Int | String) => "is"
  case g: (Int => Int) @unchecked => s"fn ${g(2)}"
  case () => "unit"
  case (a, b) => s"pair $a $b"
  case _ => "other"

def g(x: Int | String | Unit): String = x match
  case () => "unit"
  case v: (Int) => s"int $v"
  case s: String => s

@main def run(): Unit =
  println(f(1))
  println(f("s"))
  println(f((x: Int) => x + 1))
  println(f(()))
  println(f((1, "b")))
  println(f(2.5))
  println(g(()))
  println(g(3))
  println(g("str"))
