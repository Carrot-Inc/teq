package probe
class Ann extends scala.annotation.StaticAnnotation
trait A
trait B
trait C
object Forms:
  type Alias = Option[Int]
  val direct: Option[Int] = None
  val alias: Alias = None
  val annotated: (Option[Int] @Ann) = None
  val union: A | B | C = ???
  val intersection: A & B & C = ???
  type Elem[X] = X match
    case List[t] => t
    case _ => X
  val poly: [T] => T => T = [T] => (x: T) => x
  val context: Int ?=> String = "s"
  val lambda: ([T] =>> List[T])[Int] = Nil
  type HK = [T] =>> List[T]
  opaque type Opaque = Int
  val opaqueValue: Opaque = 1
  trait Box { type Out; val out: Out }
  def dependent(b: Box)(v: b.Out): b.Out = v
  def byName(x: => Option[Int]): Int = x.get
  def repeated(xs: Int*): Int = xs.size
  def nilary: Option[Int] = None
  def empty(): Option[Int] = None
  val existential: List[? <: A] = Nil
  val structural: A { type T; def m(x: Int): String } = ???
  def locals: Int =
    val first = { val inner = 1; inner }
    val second = 2
    first + second
  val result = alias.get + direct.get + annotated.get + nilary.get + empty().get
class Self:
  self =>
  def me: self.type = self
class Anon:
  val value = new A {}
object Synth:
  given Ordering[Int] = Ordering.Int
  extension (x: Int) def doubled(y: Int = 1): Int = x * 2 + y
  def bound[T: Ordering](t: T): T = t
  def default(x: Int = 1): Int = x
enum E:
  case First, Second
