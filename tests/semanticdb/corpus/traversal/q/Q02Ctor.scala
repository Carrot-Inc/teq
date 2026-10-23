package q02
class B[T <: AnyVal](xs: List[T], val n: scala.Int)
class E()
class F private ()
final class G extends B[Int](Nil, 1)
class H:
  self =>
  def k = 1
case class D(x: Int = Def.v)
object Def:
  val v = 1
class I[T] {}
