// scala.collection.mutable.SortedSet and TreeSet over a sorted array (cats' `distinct` and the
// application's sorted sets): add, remove, ranges, a reversed ordering, map with an ordering.
import scala.collection.mutable
object Main:
  def p(xs: Any*): Unit = println(xs.mkString(" "))
  def main(args: Array[String]): Unit =
    val s = mutable.TreeSet.empty[Int]
    p(s.add(3), s.add(1), s.add(3), s)
    s += 2
    s ++= List(5, 0)
    p(s, s.size, s.contains(2), s(9), s.head, s.last, s.toList)
    p(s.remove(5), s.remove(9), (s -= 0), s.rangeFrom(2), s.rangeUntil(2), s.minAfter(2), s.maxBefore(2))
    val t = mutable.SortedSet(3, 1, 2)(using Ordering[Int].reverse)
    p(t, t.map(_ * 2), t.filter(_ > 1), t.iterator.toList, mutable.SortedSet.empty[String].isEmpty)
    p(mutable.TreeSet("b", "a").mkString(","), (mutable.TreeSet(1, 2) == mutable.TreeSet(2, 1)), mutable.TreeSet(1).clone())
