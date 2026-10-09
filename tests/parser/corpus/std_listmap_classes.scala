// ListMap, TreeSet and TreeMap are classes of their own, as in scala-library: a type class with
// an instance per collection class picks by the static type, and their operations keep the class.
import scala.collection.immutable.{ListMap, SortedMap, SortedSet, TreeMap, TreeSet}

trait Show[A]:
  def show(a: A): String
object Show:
  given map[K, V]: Show[Map[K, V]] = m => "map:" + m.size
  given listMap[K, V]: Show[ListMap[K, V]] = m => "listMap:" + m.size
  given sortedSet[A]: Show[SortedSet[A]] = s => "sortedSet:" + s.size
  given treeSet[A]: Show[TreeSet[A]] = s => "treeSet:" + s.size
  given sortedMap[K, V]: Show[SortedMap[K, V]] = m => "sortedMap:" + m.size
  given treeMap[K, V]: Show[TreeMap[K, V]] = m => "treeMap:" + m.size

def show[A](a: A)(using s: Show[A]): String = s.show(a)

object Main:
  def main(args: Array[String]): Unit =
    val lm = ListMap("b" -> 2, "a" -> 1)
    val m: Map[String, Int] = lm
    println(show(lm))
    println(show(m))
    println(show(Map("x" -> 1)))
    println(lm)
    println(lm.updated("c", 3))
    println((lm + ("d" -> 4)).removed("b"))
    println(lm == Map("a" -> 1, "b" -> 2))
    println(ListMap.empty[Int, Int].isEmpty)
    println(ListMap.from(List(1 -> "a", 2 -> "b")).keys.toList)
    println(lm.map((k, v) => (k, v * 10)))
    val ts = TreeSet(3, 1, 2)
    val ss: SortedSet[Int] = ts
    println(show(ts))
    println(show(ss))
    println(show(SortedSet(5, 4)))
    println(ts + 0)
    println(ss.rangeFrom(2))
    println(ts == Set(1, 2, 3))
    val tm = TreeMap("b" -> 2, "a" -> 1)
    val sm: SortedMap[String, Int] = tm
    println(show(tm))
    println(show(sm))
    println(show(SortedMap("k" -> 0)))
    println(tm)
    println(sm.updated("c", 3))
    println(TreeMap.empty[Int, Int].isEmpty)
    println((SortedSet.newBuilder[Int] += 3 += 1 += 3).result())
    println((SortedMap.newBuilder[String, Int] += ("b" -> 2) += ("a" -> 1)).result())
    val big = ListMap("a" -> 1, "b" -> 2, "c" -> 3, "d" -> 4)
    println(big.takeRight(2).updatedWith("d")(_.map(_ * 10)).filter(_._2 > 1))
    println(List(big.take(1), big.drop(3), big.tail.init, big.slice(1, 3), big.dropRight(3)).mkString(" "))
    val tm2: TreeMap[Int, Int] = TreeMap(1 -> 2, 3 -> 4) ++ TreeMap(5 -> 6)
    val ts2: TreeSet[Int] = TreeSet(1) ++ TreeSet(2) + 3 - 1
    println(tm2.toString + " " + (tm2 + (7 -> 8)).removed(1) + " " + ts2)
