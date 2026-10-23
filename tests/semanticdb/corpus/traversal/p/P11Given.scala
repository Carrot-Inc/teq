package p11
trait Ord[T]:
  def cmp(a: T, b: T): Int
given Ord[Int] with
  def cmp(a: Int, b: Int) = a - b
given intOrd2: Ord[String] with
  def cmp(a: String, b: String) = 0
given Ord[Long] = new Ord[Long] { def cmp(a: Long, b: Long) = 0 }
given aliasG: Ord[Double] = ???
given [T](using o: Ord[T]): Ord[List[T]] with
  def cmp(a: List[T], b: List[T]) = 0
given listOrd[T: Ord]: Ord[Option[T]] = ???
given Ord[Char]:
  def cmp(a: Char, b: Char) = 0
given [T: Ord] => Ord[Vector[T]]:
  def cmp(a: Vector[T], b: Vector[T]) = 0
object Use:
  def sort[T](xs: List[T])(using o: Ord[T]) = xs
  val s = sort(List(1))
  val t = sort(List(1))(using summon[Ord[Int]])
  def ctx[T: Ord](x: T) = summon[Ord[T]].cmp(x, x)
  import scala.language.implicitConversions
  implicit def conv(s: String): Int = s.length
  val n: Int = "abc"
  given Conversion[Boolean, Int] = b => 1
  val bb: Int = true
  def loc: Int =
    given lg: Ord[Byte] = ???
    given Ord[Short] = ???
    sort(List(1.toByte)).size
