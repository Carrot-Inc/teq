// The type tests a match writes, by dotty's `TypeTestsCasts.transformIsInstanceOf`: where the scrutinee's
// own static type conforms to the tested type, the test is a null test (`testNotNull`), else the type's
// test. Each shape with values that tell the two apart (null, a value of another type):
// - a catch of `Throwable`: the caught value is never null;
// - a typed field or tuple element over `String` (a null test) and over `Any` (the class's test);
// - `Failure(e: Throwable)`, whose `exception` may be null;
// - a tuple pattern over a tuple (a null test before the elements);
// - an inline body's test of its type parameter, erased where a caller's type parameter fills it;
// - an array pattern over an array of a type parameter, which conforms only by the case's refinement.
// tests/cases/type_test_null_shapes_js throws raw JavaScript values into the catch; the element kinds of
// arrays are tests/cases/pattern_array_gadt's (the JVM's). scalac prints the lines of the .expected file.
import scala.util.{Failure, Success, Try}
case class Text(text: String)
case class Loose(value: Any)
inline def check[P](d: Any): Boolean = d.isInstanceOf[P]
inline def lazyCheck[P](d: Any): () => Option[P] = () =>
  val v: Any = d
  if (v: @unchecked).isInstanceOf[P] then Some(v.asInstanceOf[P]) else None
def gen[T](d: Any): Boolean = check[T](d)
def genLazy[T](d: Any): Option[T] = lazyCheck[T](d)()
def caught(f: () => Int): String =
  try { f(); "none" } catch { case e: Throwable => "caught " + e.getMessage }
def field(x: Any): String = x match
  case Text(s: String) => "s:" + s
  case _ => "other"
def looseField(x: Any): String = x match
  case Loose(s: String) => "s:" + s
  case _ => "other"
def names(xs: Seq[(String, Boolean)]): Seq[String] = xs.flatMap {
  case (name: String, _) => Some(name)
  case _ => None
}
def anyNames(xs: Seq[(Any, Boolean)]): Seq[String] = xs.flatMap {
  case (name: String, _) => Some(name)
  case _ => None
}
def failed(t: Try[Int]): String = t match
  case Failure(e: Throwable) => "failure"
  case _ => "other"
def pair(t: (String, Int)): String = t match
  case (x, y) => x + y
  case _ => "other"
def direct[A](array: Array[A]): String = array match
  case x: Array[Boolean] => "bool"
  case _ => "other"
def viaCast[A](array: Array[A]): String = (array.asInstanceOf[AnyRef]: @unchecked) match
  case x: Array[Boolean] => "bool"
  case _ => "other"
@main def run(): Unit =
  println(s"catch: ${caught(() => throw new IllegalStateException("x"))},${caught(() => 1)}")
  println(s"field: ${field(Text(null))},${field(Text("t"))}")
  println(s"loose: ${looseField(Loose(null))},${looseField(Loose(1))},${looseField(Loose("s"))}")
  println(s"names: ${names(Seq(("a", true), (null, false)))},${anyNames(Seq((1, true), ("s", false), (null, true)))}")
  println(s"failed: ${failed(Failure(null))},${failed(Failure(new Exception("e")))},${failed(Success(1))}")
  println(s"pair: ${pair(null)},${pair(("b", 2))}")
  println(s"gen: ${gen[String](null)},${gen[String](1)},${gen[String]("s")}")
  println(s"genLazy: ${genLazy[String](null)},${genLazy[String](1)},${genLazy[String]("s")}")
  println(s"concrete: ${lazyCheck[String](1)()},${lazyCheck[String]("s")()},${check[String](1)},${check[String](null)}")
  println(s"arrays: ${direct(null: Array[Int])},${direct(Array(true))},${viaCast(null: Array[Int])},${viaCast(Array(true))}")
