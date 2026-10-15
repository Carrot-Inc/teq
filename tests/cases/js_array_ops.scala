//> using platform js
// Scala.js's js.Array is a type of its own: its collection methods are js.ArrayOps's, building
// js.Arrays without a ClassTag (`collect` and `map` of an element type the caller abstracts over,
// as tyrian-next's and indigo's `Batch` call them), it converts to a Seq where one is expected,
// `new js.Array[A](n)` makes n empty slots, and a type test tells it from other values.
import scala.scalajs.js
import scala.scalajs.js.JSConverters.*

sealed trait Batch[+A]:
  def toJSArray[B >: A]: js.Array[B]
  def collect[B >: A, C](f: PartialFunction[B, C]): js.Array[C] = toJSArray.collect(f)
  def map[B](f: A => B): js.Array[B] = toJSArray.map(f)

final case class Wrapped[A](values: js.Array[A]) extends Batch[A]:
  def toJSArray[B >: A]: js.Array[B] = values.asInstanceOf[js.Array[B]]

def sum(xs: Seq[Int]): Int = xs.sum

@main def run(): Unit =
  val a = js.Array(1, 2, 3)
  a.push(4)
  println(a.length)
  println(a.map(_ * 2).mkString(","))
  println(Wrapped(a).collect { case x if x % 2 == 0 => x.toString }.join("-"))
  println(Wrapped(a).map(_ + 1).toList)
  println(sum(a.toSeq))
  println(new js.Array[Int](3).length)
  val c = new js.Array[String]()
  c.push("x")
  println(s"${c.length} ${js.Array.isArray(c)}")
  println(List(1, 2).toJSArray.filter(_ > 1).length)
  println((a: Any) match
    case _: js.Array[?] => "array"
    case _ => "other")
  println(a(0) + a.head + a.last)
  a(0) = 10
  println(a.toList)
  println(a.sum)
  println(s"${a.reverse.toList} ${a.toList}")
  a.reverseInPlace()
  println(a.toList)
