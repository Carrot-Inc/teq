package demo.holders

object Lib:
  def flag: Boolean = System.nanoTime() != 0L

inline def sum(flag: Boolean): Int =
  val (a, b) = if flag then (1, 2) else (3, 4)
  a + b

object Held:
  val (x, y) = if Lib.flag then (1, 2) else (3, 4)
  lazy val (u, v) = if Lib.flag then (5, 6) else (7, 8)
  val plain = x + y

@main def main(): Unit =
  println(Inspect.check {
    val (a, b) = if Lib.flag then (1, 2) else (3, 4)
    a + b
  })
  println(Inspect.check {
    lazy val (c, d) = if Lib.flag then (5, 6) else (7, 8)
    c + d
  })
  println(Inspect.check(sum(Lib.flag)))
  println(Inspect.fields[Held.type])
  println(Held.plain + Held.u)
