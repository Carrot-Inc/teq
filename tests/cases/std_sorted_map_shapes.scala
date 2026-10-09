// `SortedMap.map`, `collect` and `flatMap` give a sorted map when the function gives pairs and
// an ordering is at hand, an Iterable otherwise, as scala-library's overloads do (cats'
// `NonEmptyList.groupMap` maps a `TreeMap` with its ordering); `LazyList.cons` is an object.
import scala.collection.immutable.{SortedMap, TreeMap}
object Main:
  def main(args: Array[String]): Unit =
    val sm = SortedMap(3 -> "c", 1 -> "a", 2 -> "b")
    println(sm.map((k, v) => (v, k)))
    println(sm.map((k, v) => (-k, v)))
    println(sm.map((k, v) => v + k))
    println(sm.map[String, Int](e => (e._2, e._1))(using Ordering[String].reverse))
    println(sm.collect { case (k, v) if k > 1 => (v, k) })
    println(sm.collect { case (k, v) if k > 1 => v })
    println(sm.flatMap((k, v) => List((k * 10, v), (k * 100, v))))
    println(sm.flatMap((k, v) => List(v, v)))
    val tm = TreeMap(2 -> List(1), 1 -> List(2, 3))
    println(tm.map((k, v) => (k, v.sum)))
    println((sm: Iterable[(Int, String)]).map(_._1))
    println(LazyList.cons(1, LazyList.cons(2, LazyList.empty)).toList)
    println(LazyList.cons.apply(0, LazyList.empty[Int]).toList)
