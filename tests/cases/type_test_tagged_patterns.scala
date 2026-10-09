// A type pattern over an abstract type goes through the `TypeTest[S, T]` or the `ClassTag[T]` in
// scope (dotty's `tryWithTypeTest`, Typer.scala 1382-1404): an application's shapes, a tagged
// `case e: E` in a handler, `collect` with a tag, a nested `case Some(t: T)`, a `catch` clause, an
// abstract type member's tag, a handwritten `TypeTest`, the binder's type, and the extractor's
// abstract input type (Applications.scala 1958). Without a tag the test is the bound's.
import scala.reflect.{ClassTag, TypeTest}

trait Shape
class Circle(val r: Int) extends Shape:
  override def toString = s"Circle($r)"
class Square(val s: Int) extends Shape:
  override def toString = s"Square($s)"
class Boom(msg: String) extends RuntimeException(msg)
class Other(msg: String) extends RuntimeException(msg)

object Domain:
  opaque type Name = String
  def name(s: String): Name = s
  given TypeTest[Any, Name] with
    def unapply(x: Any): Option[x.type & Name] =
      Counts.opaque += 1
      x match
        case s: String if s.startsWith("n:") => Some(s.asInstanceOf[x.type & Name])
        case _ => None

object Counts:
  var opaque = 0
  var deny = 0
  var tags = 0

trait Codec:
  type A
  def tag: ClassTag[A]
  def read(x: Any): Option[A] =
    given ClassTag[A] = tag
    x match
      case a: A => Some(a)
      case _ => None

object Main:
  def f2[T: ClassTag](x: Any) = x match { case t: T => 1; case _ => 2 }
  def g[T: ClassTag](xs: List[Any]): List[T] = xs.collect { case t: T => t }
  // zio's `catchSome` reduced: a partial function over a tagged `E`, asked and applied.
  def catchSome[E <: Throwable: ClassTag](io: () => Int)(h: PartialFunction[E, Int]): Int =
    try io()
    catch
      case e: E if h.isDefinedAt(e) => h(e)
  // cats-effect's `recoverWith` reduced.
  def recoverWith[E <: Throwable: ClassTag](e: Throwable)(h: E => Int): Option[Int] = e match
    case e: E => Some(h(e))
    case _ => None
  def nested[T: ClassTag](x: Option[Any]) = x match
    case Some(t: T) => s"some $t"
    case Some(other) => s"other $other"
    case None => "none"
  def caught[E <: Throwable: ClassTag](e: Throwable): String =
    try
      try throw e
      catch { case _: E => "hit" }
    catch { case _: Throwable => "miss" }
  def bound[S, T](x: S)(using TypeTest[S, T]): String = x match
    case s @ (t: T) => s"bound $s $t"
    case _ => "unbound"
  def deny[S, T]: TypeTest[S, T] = new TypeTest[S, T]:
    def unapply(x: S): Option[x.type & T] =
      Counts.deny += 1
      None
  def bounded[T <: Shape](x: Any)(using TypeTest[Any, T]) = x match
    case _: T => true
    case _ => false
  def union[T](x: Any)(using TypeTest[Any, T | String]) = x match
    case _: (T | String) @unchecked => true
    case _ => false
  def applied[F[_]](x: Any)(using TypeTest[Any, F[Int]]) = x match
    case _: F[Int] => true
    case _ => false
  def broad[T](x: String)(using TypeTest[Any, T]) = x match
    case _: T => true
    case _ => false
  def narrow[T](x: Any)(using TypeTest[String, T]) = x match
    case _: T @unchecked => true
    case _ => false
  // No tag: the test is the erasure of the bound, which `null` never passes.
  def fallback[T <: Shape](x: Any) = x match
    case _: T @unchecked => true
    case _ => false
  def unbounded[T](x: Any) = x match
    case _: T @unchecked => true
    case _ => false
  def opaque(x: Any): Boolean = x match
    case _: Domain.Name => true
    case _ => false
  // The extractor's input type: the tag's test comes first, the extractor runs only after it.
  def extract[T: ClassTag](x: Any): String =
    object Extract:
      def unapply(t: T): Boolean = { Counts.tags += 1; true }
    x match
      case Extract() => "yes"
      case _ => "no"
  def main(args: Array[String]): Unit =
    println(f2[String](1)); println(f2[String]("s")); println(f2[Int](1)); println(f2[Int]("s"))
    println(g[String](List(1, "a", 2.5, "b")))
    println(g[Int](List(1, "a", 2.5, 3)))
    println(g[Shape](List(Circle(1), "a", Square(2))))
    println(catchSome[Boom](() => throw Boom("b")) { case e => 7 })
    println(try catchSome[Boom](() => throw Boom("b")) { case e if e.getMessage == "x" => 7 } catch { case e: Boom => s"undefined ${e.getMessage}" })
    println(try catchSome[Boom](() => throw Other("o")) { case e => 7 } catch { case e: Other => s"escaped ${e.getMessage}" })
    println(recoverWith[Boom](Boom("b"))(e => e.getMessage.length))
    println(recoverWith[Boom](Other("o"))(e => e.getMessage.length))
    println(nested[Circle](Some(Circle(3)))); println(nested[Circle](Some(Square(3)))); println(nested[Circle](None))
    println(caught[Boom](Other("x"))); println(caught[Boom](Boom("x")))
    println(bound[Any, String]("s")(using summon[TypeTest[Any, String]]))
    println(bound[Any, String](1)(using summon[TypeTest[Any, String]]))
    println(s"${bounded[Circle](Circle(1))(using deny)} ${Counts.deny}")
    println(s"${union[Int](1)(using deny)} ${Counts.deny}")
    println(s"${applied[List](List(1))(using deny)} ${Counts.deny}")
    println(s"${broad[String]("x")(using deny)} ${Counts.deny}")
    println(s"${narrow[String](1)(using deny)} ${Counts.deny}")
    println(fallback[Circle](Circle(1))); println(fallback[Circle]("s")); println(fallback[Circle](null))
    println(unbounded[String](1)); println(unbounded[String](null))
    println(opaque("n:x")); println(opaque("x")); println(opaque(1)); println(Counts.opaque)
    println(s"${extract[String](1)} ${Counts.tags}"); println(s"${extract[String]("ok")} ${Counts.tags}")
    object IntCodec extends Codec:
      type A = Int
      def tag: ClassTag[A] = summon[ClassTag[Int]]
    println(IntCodec.read(1)); println(IntCodec.read("s"))
