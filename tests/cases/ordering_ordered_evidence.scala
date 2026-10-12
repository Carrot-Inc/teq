// Ordering.ordered takes scala-library's evidence that the type is comparable (AsComparable).
class V(val n: Int) extends Comparable[V]:
  def compareTo(o: V): Int = n - o.n

@main def main(): Unit =
  val vs = List(V(3), V(1), V(2))
  println(vs.sorted(using Ordering.ordered[V]).map(_.n))
  println(vs.sorted(using Ordering.ordered[V](using (v: V) => v)).map(_.n))
  println(vs.max(using Ordering.ordered[V]).n)
