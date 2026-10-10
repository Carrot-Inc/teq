// scala-library's members that libraries built for 2.12 and 2.13 still call, deprecated or not,
// which teq's std holds for their bodies (ScalaTest's and scalactic's on Scala.js among them).
import scala.collection.mutable

case class V(n: Int) extends Ordered[V]:
  def compare(o: V): Int = n - o.n

@main def run(): Unit =
  val s: scala.collection.Set[Int] = Set(1, 2)
  println((s + 3).toList.sorted)
  println(scala.collection.Set.empty[Int].size)
  val gm: scala.collection.GenMap[String, Int] = Map("a" -> 1, "b" -> 2)
  println(gm.filter(_._2 > 1))
  println(gm.tail.size)
  val gs: scala.collection.GenSeq[Int] = List(1, 2)
  println(gs)
  println(List(1, 2, 3)./:(0)(_ + _))
  println(s"${List(1).hasDefiniteSize} ${Iterator(1).hasDefiniteSize} ${Iterator().hasDefiniteSize}")
  val it = List("a", "b", "c").iterator.buffered
  println(it.head + it.next() + it.head + it.headOption)
  println("a.b.c".replaceAllLiterally(".", "-"))
  println(List(V(2), V(1)).sorted(using Ordering.ordered[V]))
  println(scala.compat.Platform.currentTime > 0)
  val m = mutable.HashMap(1 -> "a") ++ List(2 -> "b")
  m(3) = "c"
  println(m.toList.sorted)
  println(new StackTraceElement("C", "m", "C.scala", 1).isNativeMethod)
  val seq = new scala.collection.AbstractSeq[Int] with IndexedSeq[Int]:
    def length = 2
    def apply(i: Int) = i * 10
  println(seq.toList)
  println(s"${(Array(1): Any).isInstanceOf[scala.collection.ArrayOps[?]]} ${("s": Any).isInstanceOf[scala.collection.StringOps]}")
