// The java.lang.Number members of the numeric types, scan on every collection with scanLeft, and
// SortedSet's map and flatMap, which keep the sorted set where an Ordering for the result exists
// and give a Set through the static type Set.

import scala.collection.immutable.SortedSet
@main def main(): Unit =
  val d: Double = 3.7
  println(d.intValue)
  println(d.longValue)
  println(d.doubleValue)
  val i: Int = 5
  println(i.longValue)
  println(i.doubleValue / 2)
  println(i.intValue)
  val l: Long = 7L
  println(l.intValue)
  println(l.doubleValue / 2)
  println(l.longValue)
  println(List(1, 2, 3).scan(0)(_ + _))
  println(List(1, 2, 3).scanRight(0)(_ + _))
  println(Vector(1, 2, 3).scan(0)(_ + _))
  println(Seq(1, 2, 3).scan(0)(_ + _))
  val s = SortedSet(3, 1, 2)
  val m = s.map(_ * 2)
  println(m)
  val f = s.flatMap(x => SortedSet(x, x + 10))
  println(f)
  val g = s.filter(_ > 1)
  println(g)
  val ss: SortedSet[Int] = m
  val fs: SortedSet[Int] = f
  val gs: SortedSet[Int] = g
  println(ss.min + fs.max + gs.size)
  println(s.map(_.toString))
  println(s.map(x => (x % 2, x)))
  val plain: Set[Int] = s
  println(plain.map(_ * 10).toList.sorted)
  println(plain.flatMap(x => List(x, -x)).toList.sorted)
