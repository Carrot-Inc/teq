// jars: scala-library
// std: lean scala-library
// A varargs literal of value class instances holds the instances, whatever the class holds:
// scala-library's sequence gives them back as the class.
final class V(val x: Int) extends AnyVal
final class W(val d: Double) extends AnyVal:
  def twice: Double = d * 2
def first(xs: V*): Int = xs.head.x
def sum(ws: W*): Double = ws.map(_.twice).sum
def all[T](xs: T*): Int = xs.length

@main def run(): Unit =
  println(first(new V(7)))
  println(sum(new W(1.5), new W(2)))
  println(all(new V(1), new V(2)) + all(1, 2.5) + all("s"))
  val vs: Seq[V] = Seq(new V(3), new V(4))
  println(vs.map(_.x).sum.toString + " " + first(vs*))
