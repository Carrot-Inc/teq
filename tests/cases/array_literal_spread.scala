// `Array(x, xs*)` with a spread: the overload is the one the sequence's elements fit, the
// head widened to them (`Array(1, longs*)` is an `Array[Long]`), and nothing else.
import scala.reflect.ClassTag

def described[T](a: Array[T])(using ct: ClassTag[T]): String = ct.toString + " " + a.mkString(",")

@main def run(): Unit =
  val longs: Seq[Long] = Seq(2L, 3L)
  println(described(Array(1, longs*)))
  val ints = List(2, 3)
  println(described(Array(1, ints*)) + " " + described(Array(1L, ints.map(_.toLong)*)))
  val doubles = Vector(2.5)
  println(Array(1, doubles*).map(x => (x * 10).toInt).mkString(",") + " " + described(Array(1, doubles*)).take(6))
  println(described(Array(true, List(false)*)) + " " + described(Array('a', List('b')*)))
