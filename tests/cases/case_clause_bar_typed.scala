// Typed alternatives across a trailing `|`, alternatives of the case and not a union type (dotty's `InCase`).
def f(x: Any): Int = x match
  case _: Int |
      _: String => 1
  case _ => 0
@main def run(): Unit = println(f(1))
