// jars: extend-lib
// std: lean scala-library
// targets: js interp jvm
// A scalac-built library's classes extending PartialFunction and ClassTag, and its view results,
// consumed here: the class instance is a function and a partial function, its overrides and the
// tag's are dispatched, a program's subclass of the library's base class too, and the views keep
// scala-library's types.
import scala.collection.{View, SeqView, IndexedSeqView}
import scala.reflect.ClassTag
import extendlib.*

class Twice extends Base:
  override def apply(x: String): Int = 2 * x.length

def make[T](n: Int)(using t: ClassTag[T]): Array[T] = new Array[T](n)

@main def run(): Unit =
  val p: PartialFunction[Int, Int] = new Offset(10)
  val f: Int => Int = p
  println(f(1))
  println(p.applyOrElse(0, (x: Int) => -x))
  println(List(-1, 2).collect(p))
  println((p orElse { case x => 0 })(-5))
  println(p.lift(3))
  val b: PartialFunction[String, Int] = new Twice
  println(List("", "ab").collect(b))
  val t: ClassTag[String] = new Tag
  println(t.unapply(1))
  println(make[String](2)(using t).toList)
  println(List[Any](1, "x").map { case t(s) => s; case other => "plain" })
  val s: SeqView[Int] = Views.seq
  val i: IndexedSeqView[Int] = Views.indexed
  val m: SeqView[Int] = Views.mapped(s)
  val v: View[Int] = Views.filtered(i)
  println(List(s, i, m, v).map(_.toList))
  println(Views.mapped(i.reverse).toList)
