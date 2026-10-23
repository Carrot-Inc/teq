package boundorder
class Out[+A]
object Lower:
  def pick[A](x: Out[A]): Out[A] = ???
  val one = pick(??? : Out[Int @First])
  def join[A](x: Out[A], y: Out[A]): Out[A] = ???
  val two = join(??? : Out[Int @First], ??? : Out[Int @Second])
