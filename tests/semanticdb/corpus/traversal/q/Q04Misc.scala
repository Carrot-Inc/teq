package q04
trait Box { type Out; val out: Out }
class Holder[F[_]]
object Misc:
  def dep(b: Box): b.Out = b.out
  def poly[F[_], G[_, _]](f: F[Int]): F[Int] = f
  def struct(x: { def a: Int }): Int = 1
  def hk(h: Holder[[T] =>> List[T]]): Int = 1
  def ret(i: Int): Int =
    if i > 0 then return 1
    while i < 0 do ()
    i
  def tup(t: (Int, String)): Int = t match
    case (a, b) => a
  def g = (1, "x") match { case (y, _) => y }
