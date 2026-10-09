package scala2lib

trait Encoder[E, D, T] { self =>
  def encode(d: D): E
  def contramap[DD](f: DD => D): Encoder[E, DD, T] = Encoder.from(f andThen self.encode)
}

object Encoder {
  def from[E, D, T](f: D => E): Encoder[E, D, T] = new Encoder[E, D, T] { def encode(d: D): E = f(d) }
}

trait EncoderCompanion[E, T] {
  def apply[D](implicit ev: Encoder[E, D, T]): Encoder[E, D, T] = ev
  def from[D](f: D => E): Encoder[E, D, T] = Encoder.from(f)
}

trait CellInstances {
  implicit val stringCell: Encoder[String, String, codecs.type] = Encoder.from(identity)
  implicit val intCell: Encoder[String, Int, codecs.type] = Encoder.from(_.toString)
  implicit def optionCell[A](implicit a: Encoder[String, A, codecs.type]): Encoder[String, Option[A], codecs.type] =
    Encoder.from(_.fold("")(a.encode))
}

object codecs extends CellInstances

object Cell extends EncoderCompanion[String, codecs.type]

final class Rows[A](val as: Iterable[A])(implicit e: Cell[A]) {
  def asLine(sep: Char, header: String*): String = (header ++ as.map(e.encode)).mkString(sep.toString)
}

trait ToRowsOps {
  implicit def toRowsOps[A: Cell](as: Iterable[A]): Rows[A] = new Rows(as)
}

package object ops extends ToRowsOps

sealed abstract class Shape(val sides: Int) extends Product with Serializable
case class Square(side: Double) extends Shape(4)
case object Dot extends Shape(0)

object Settings {
  implicit val defaultSep: Char = ';'
  final val Version = 3
  var counter: Int = 0
  lazy val greeting: String = "hi"
}

final case class Exported[A](value: A) extends AnyVal

trait Greeter[A] { def greet(a: A): String }

object Greeter {
  implicit def fromExported[A](implicit e: Exported[Greeter[A]]): Greeter[A] = e.value
}

object greeters {
  implicit val exportedIntGreeter: Exported[Greeter[Int]] = Exported(new Greeter[Int] { def greet(a: Int): String = s"hello $a" })
}

final case class Version private (major: Int, minor: Int)
object Version {
  def of(major: Int): Version = new Version(major, 0)
}

final class Guarded private (val n: Int)
object Guarded {
  def make(n: Int): Guarded = new Guarded(n)
}

case class Edge(from: String, to: String)(val weight: Int)

// A trait whose class file names a private val's accessors, which the pickle's transcoding leaves out
// (tests/classpath/jvm/scala2_trait_fields).
trait Settings {
  val name: String = "settings"
  private val secret: Int = 42
  def reveal: Int = secret + 1
}
