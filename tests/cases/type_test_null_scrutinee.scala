// The null test in front of the tag's extractor is decided as dotty's `isNotNull` decides it
// (Types.scala 384-395; PatternMatcher.scala 495-496): a scrutinee typed `S | Int`, `S & Product`
// over an abstract `S`, an opaque type over a reference or a type whose bound admits `null` is
// tested, so the evidence is never called for a `null`, at a type pattern and at an extractor
// whose `unapply` takes the abstract type alike; a value type, a literal type or an opaque type
// over a value in its scope is not tested, so the evidence runs.
import scala.reflect.TypeTest

object O:
  opaque type Text = String
  def empty: Text = null.asInstanceOf[Text]
  def some: Text = "t"
  opaque type Num = Int
  def num: Num = 7
  def inScope[T](x: Num)(using TypeTest[Num, T]) = x match { case _: T => true; case _ => false }

object Counts:
  var calls = 0

object Main:
  def deny[S, T]: TypeTest[S, T] = new TypeTest[S, T]:
    def unapply(x: S): Option[x.type & T] = { Counts.calls += 1; None }
  def accept[S, T]: TypeTest[S, T] = new TypeTest[S, T]:
    def unapply(x: S): Option[x.type & T] = { Counts.calls += 1; Some(x.asInstanceOf[x.type & T]) }
  def union[S, T](x: S | Int)(using TypeTest[S | Int, T]): Boolean = x match
    case _: T => true
    case _ => false
  def extractor[S, T](x: S | Int)(using TypeTest[S | Int, T]): Boolean =
    object Extract:
      def unapply(x: T): Boolean = { Counts.calls += 10; true }
    x match
      case Extract() => true
      case _ => false
  def opaque[T](x: O.Text)(using TypeTest[O.Text, T]) = x match
    case _: T => true
    case _ => false
  def inter[S <: AnyRef, T](x: S & Product)(using TypeTest[S & Product, T]) = x match
    case _: T => true
    case _ => false
  def withNull[S, T](x: S | Null)(using TypeTest[S | Null, T]) = x match
    case _: T => true
    case _ => false
  def value[S <: AnyVal, T](x: S)(using TypeTest[S, T]) = x match
    case _: T => true
    case _ => false
  def bounded[S <: String, T](x: S)(using TypeTest[S, T]) = x match
    case _: T => true
    case _ => false
  def main(args: Array[String]): Unit =
    def show(label: String, v: Boolean): Unit = { println(s"$label=$v calls=${Counts.calls}"); Counts.calls = 0 }
    show("union-null", union[String, String](null)(using accept))
    show("union-value", union[String, String]("s")(using accept))
    show("extractor-null", extractor[String, String](null)(using accept))
    show("extractor-value", extractor[String, String]("s")(using accept))
    show("opaque-null", opaque[String](O.empty)(using deny))
    show("opaque-value", opaque[String](O.some)(using deny))
    show("inter-null", inter[AnyRef, String](null)(using deny))
    show("with-null", withNull[String, String](null)(using deny))
    show("value", value[Int, String](1)(using deny))
    show("bounded-null", bounded[String, String](null)(using deny))
    show("in-scope", O.inScope[String](O.num)(using deny))
