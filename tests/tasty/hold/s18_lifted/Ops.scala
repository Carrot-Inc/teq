import scala.language.implicitConversions
class Ops[A](a: A):
  def m(b: A) = Pair.pair(y = { Pair.n += 1; Seq(b) }, x = { Pair.n += 10; Seq(a) }).head.head
object Pair:
  var n = 0
  def pair[T](x: T, y: T): List[T] = List(x, y)
