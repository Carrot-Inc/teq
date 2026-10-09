// Several arguments passed as a tuple where two alternatives take one parameter (http4s'
// `+?[K, T](param: (K, T))` beside `+?[K](name: K)`): the most specific of them is chosen,
// as dotc resolves the overload again with the arguments tupled.
trait KeyLike[K]
object KeyLike:
  given KeyLike[String] = new KeyLike[String] {}
trait Enc[T]
object Enc:
  given Enc[Int] = new Enc[Int] {}
trait QP[T]
class Q(val s: String):
  def +?[T: QP]: Q = this
  def +?[K: KeyLike, T: Enc](param: (K, T)): Q = Q(s + "&" + param._1 + "=" + param._2)
  def +?[K: KeyLike](name: K): Q = Q(s + "&" + name)
object Main:
  def main(args: Array[String]): Unit =
    val q = Q("x") +? ("a", 1) +? ("b", 2)
    println(q.s)
