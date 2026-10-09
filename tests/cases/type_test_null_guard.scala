// The tag's extractor runs behind a test that the scrutinee is not `null`, as dotc's pattern
// matcher guards an extractor (PatternMatcher.scala 495-496, `NonNullTest`): the evidence is not
// evaluated and its `unapply` not called for a `null`, which matches no type pattern, the
// identity `TypeTest` included; a scrutinee that cannot be `null` has no test.
import scala.reflect.{ClassTag, TypeTest}

object Counts:
  var calls = 0

object Main:
  def f[T](x: Any)(using TypeTest[Any, T]): Boolean = x match
    case _: T => true
    case _ => false
  def g[T: ClassTag](x: Any): Boolean = x match
    case _: T => true
    case _ => false
  def h[T](x: Int)(using TypeTest[Int, T]): Boolean = x match
    case _: T => true
    case _ => false
  def collectAll[T](xs: List[Any])(using TypeTest[Any, T]): List[T] = xs.collect { case t: T => t }
  def main(args: Array[String]): Unit =
    val tag = new TypeTest[Any, String]:
      def unapply(x: Any): Option[x.type & String] = { Counts.calls += 1; None }
    println(f[String](null)(using tag)); println(Counts.calls)
    println(f[String]("s")(using tag)); println(Counts.calls)
    println(f[Any](null)(using TypeTest.identity[Any])); println(f[Any](1)(using TypeTest.identity[Any]))
    println(g[String](null)); println(g[String]("s"))
    println(h[Int](3)(using TypeTest.identity[Int]))
    println(collectAll[String](List("a", null, 1, "b")))
    println(collectAll[Any](List("a", null, 1))(using TypeTest.identity[Any]))
