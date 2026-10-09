import scala.collection.immutable.{SortedMap, TreeMap, IntMap}
import scala.collection.mutable.PriorityQueue
class Foo(val n: Int) extends Comparable[Foo]:
  def compareTo(that: Foo): Int = n - that.n
@main def main(): Unit =
  println(f"${123.456}%g ${1e10}%g ${0.0001234}%g ${0.00001234}%g ${0.0}%g ${-2.5}%.3g ${123456789.0}%g ${100.0}%g")
  println(s"${Math.rint(2.5).toInt} ${Math.rint(3.5).toInt} ${Math.rint(-2.5).toInt} ${Math.rint(2.4).toInt} ${math.rint(0.5).toInt} ${Math.rint(-0.5).toInt} ${Math.rint(7.0).toInt}")
  println(Double.MinPositiveValue > 0)
  println(Integer.parseInt("42") + Integer.parseInt("ff", 16) + Integer.MAX_VALUE)
  println(s"${Integer.toHexString(255)} ${Integer.toString(255, 2)} ${Integer.toBinaryString(5)} ${String.valueOf(12)} ${String.format("%d-%s", 1, "a")} ${Integer.MIN_VALUE}")
  println(s"${255.toHexString} ${(-1).toHexString} ${255L.toHexString} ${(-1L).toHexString} ${8.toOctalString} ${5.toBinaryString} ${(-8L).toBinaryString.length}")
  println(s"${'a'.compare('b')} ${'a' max 'b'} ${'a' min 'b'} ${'z'.compare('a') > 0}")
  val m = Map(1 -> "a").withDefaultValue("z")
  println(s"${m(1)}${m(2)} ${m.get(2)} ${(m + (3 -> "c"))(4)} ${m.contains(2)} ${m.getOrElse(2, "q")} ${(m - 1)(1)} ${m.filter(_ => true)(7)}")
  val md = Map(1 -> 10).withDefault(k => k * 100)
  println(md(1) + md(5))
  println(s"${"abc".zip("xyz".toList)} ${"abcd".intersect("bdx")} ${"abcd".diff("bd")} ${"ab".zip(List(1, 2, 3))}")
  val sb = new StringBuilder("hello")
  println(s"${sb.dropRight(1)} ${sb.take(2)} ${sb.drop(3)} ${sb.head} ${sb.last} ${sb.startsWith("he")} ${sb.contains("ell")} ${sb.toList}")
  println(s"${Array.range(1, 5).toList} ${Array.range(1, 10, 3).toList} ${Array((1, "a"), (2, "b")).unzip._2.toList} ${Array((1, "a"), (2, "b")).unzip._1.sum}")
  val sm = SortedMap(3 -> "c", 1 -> "a", 2 -> "b")
  println(s"$sm ${sm(2)} ${sm.get(5)} ${sm + (0 -> "z")} ${sm - 1} ${sm.keys} ${sm.values.toList} ${sm.firstKey} ${sm.lastKey}")
  println(s"${sm.filterKeys(_ > 1).toList} ${sm.rangeFrom(2)} ${sm.rangeUntil(2)} ${sm.equals(Map(1 -> "a", 2 -> "b", 3 -> "c"))} ${TreeMap("b" -> 1, "a" -> 2)} ${sm.map((k, v) => v + k)} ${sm.toList} ${sm.head} ${sm.size} ${sm.contains(3)} ${(sm ++ List(5 -> "e", 1 -> "A")).toList} ${sm.updated(2, "B")}")
  println(s"${IntMap(1 -> "a").size} ${IntMap.empty[String].size} ${IntMap(1 -> "a")(1)}")
  val pq = PriorityQueue(3, 1, 4, 1, 5, 9, 2, 6)
  println(pq)
  println(s"${pq.dequeue()} ${pq.dequeue()} ${pq.head} ${pq.size} ${pq.dequeueAll} ${pq.isEmpty}")
  val pq2 = PriorityQueue.empty[Int]
  pq2 += 5
  pq2.enqueue(1, 7)
  pq2 ++= List(3, 8)
  println(s"$pq2 ${pq2.toList} ${pq2.dequeueAll}")
  println(PriorityQueue("b", "a", "c")(using Ordering.String.reverse).dequeueAll)
  println(PriorityQueue(1, 2, 3, 4, 5, 6, 7, 8, 9, 10).dequeueAll)
  val ll = LazyList.from(1).map(_ * 2)
  println(ll)
  println(ll.take(3).toList)
  println(ll)
  lazy val fibs: LazyList[Int] = LazyList.cons(0, LazyList.cons(1, fibs.zip(fibs.tail).map((a, b) => a + b)))
  println(fibs.take(10).toList)
  println(s"${LazyList(1, 2, 3).map(_ + 1).filter(_ > 2).toList} ${LazyList.continually(7).take(2).toList} ${LazyList.iterate(1)(_ * 3).take(4).toList} ${(LazyList(1) ++ LazyList(2)).toList} ${LazyList.range(1, 4).toList} ${LazyList.fill(2)("x").toList} ${LazyList.tabulate(3)(_ * 2).toList} ${LazyList.unfold(1)(s => if s > 3 then None else Some((s, s + 1))).toList} ${LazyList.empty[Int].isEmpty} ${LazyList(1, 2).headOption}")
  val evens = LazyList.from(1).filter(_ % 2 == 0)
  println(s"${evens.head} ${evens.drop(2).head} ${LazyList.from(1).takeWhile(_ < 4).toList} ${LazyList.from(1).dropWhile(_ < 4).head} ${LazyList(1, 2, 3).force} ${LazyList(1, 2, 3).scanLeft(0)(_ + _).toList} ${LazyList(1, 2).zipWithIndex.toList} ${(1 #:: 2 #:: LazyList.empty).toList} ${LazyList(1, 2, 3).mkString(",")} ${LazyList.from(1, 3).take(3).toList}")
  var evaluated = 0
  val counted = LazyList.from(1).map: x =>
    evaluated += 1
    x
  println(s"${counted.take(3).toList} ${counted.take(3).toList} $evaluated")
  println(new Foo(2).compareTo(new Foo(1)))
