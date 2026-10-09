class R(val n: Int)
inline def use(using r: R): Int = summon[R].n
@main def main(): Unit =
  println(use(using new R(1)))
  println(use(using new R(2)))
