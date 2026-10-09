class R(val n: Int)
trait T:
  def r: R
  given g: R = r
  inline def m: Int = summon[R].n
object P extends T { def r = new R(1) }
object Q extends T { def r = new R(2) }

@main def main(): Unit =
  println(P.m)
  println(Q.m)
