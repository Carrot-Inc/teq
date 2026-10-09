// `map(f)(g)` next to `map(mapping)`, where `Mapping` is a trait with several abstract methods:
// a function literal cannot stand for it, so the shape pass rules that alternative out
// (dotc's `narrowByShapes`) and the two-list one is chosen, also through an override.
trait Mapping[L, H]:
  def rawDecode(l: L): Option[H]
  def encode(h: H): L
  def validator: List[String]
  def decode(l: L): Option[H] = rawDecode(l)
object Mapping:
  def from[L, H](f: L => H)(g: H => L): Mapping[L, H] = new Mapping[L, H]:
    def rawDecode(l: L): Option[H] = Some(f(l))
    def encode(h: H): L = g(h)
    def validator: List[String] = Nil

sealed trait Transput[T]:
  type ThisType[X] <: Transput[X]
  def map[U](mapping: Mapping[T, U]): ThisType[U]
  def map[U](f: T => U)(g: U => T): ThisType[U] = map(Mapping.from(f)(g))
  def show: String

sealed trait Input[T] extends Transput[T]:
  type ThisType[X] <: Input[X]
object Input:
  sealed trait Single[T] extends Input[T]:
    type ThisType[X] <: Single[X]
  sealed trait Basic[T] extends Single[T]:
    type ThisType[X] <: Basic[X]
    def copyWith[U](name: String): ThisType[U]
    override def map[U](mapping: Mapping[T, U]): ThisType[U] = copyWith[U](show + ".mapped")
  case class Query[T](name: String) extends Basic[T]:
    override type ThisType[X] = Query[X]
    override def copyWith[U](n: String): Query[U] = Query(n)
    def show: String = name

case class Wrapped(s: String)

object Main:
  def main(args: Array[String]): Unit =
    val q = Input.Query[String]("q")
    val m = q.map(Wrapped(_))(_.s)
    println(m.show)
    val b: Input.Basic[String] = q
    println(b.map(Wrapped(_))(_.s).show)
