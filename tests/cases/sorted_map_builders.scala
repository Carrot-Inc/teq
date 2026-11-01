// scala-library's `TreeMapBuilder` (over `MapHelper.mutableUpd`), branch by branch: insertions compared as its
// red-black tree compares them (the comparisons logged), a key found taking the new value, `++=` of a map of the same
// ordering adopting its tree into an empty builder (no comparison) and uniting with it otherwise, another map's
// entries added through its own `foreachEntry` and committed once all are in, a result leaving the builder as it was;
// and `StrictOptimizedSortedMapOps`'s `map`, `flatMap` and `collect` adding each result to that builder as it is made,
// before the next entry is taken.
import scala.collection.immutable.{SortedMap, TreeMap}
import scala.collection.mutable

object Main:
  def attempt(r: => Any): Unit =
    try println(r)
    catch case e: Exception => println(e.getMessage)
  def main(args: Array[String]): Unit =
    val log = mutable.ArrayBuffer.empty[String]
    val logged: Ordering[Int] = new Ordering[Int]:
      def compare(x: Int, y: Int): Int =
        log += s"$x?$y"
        Integer.compare(x, y)
    val b = TreeMap.newBuilder[Int, String](using logged)
    b ++= List(5 -> "e", 1 -> "a", 4 -> "d", 2 -> "b", 3 -> "c", 1 -> "A")
    println(s"${b.result()} ${log.mkString(" ")}")

    var allowed = true
    val strict: Ordering[Int] = new Ordering[Int]:
      def compare(x: Int, y: Int): Int =
        if !allowed then throw new IllegalStateException("comparison")
        Integer.compare(x, y)
    val src = TreeMap(1 -> "a", 2 -> "b", 3 -> "c")(using strict)
    allowed = false
    val adopting = TreeMap.newBuilder[Int, String](using strict)
    attempt { adopting ++= src; adopting.result().toList }
    allowed = true
    adopting += 0 -> "z"
    println(s"${adopting.result()} $src")
    allowed = false
    val updating = TreeMap.newBuilder[Int, String](using strict)
    updating ++= src
    allowed = true
    updating += 2 -> "B"
    println(s"${updating.result()} $src")
    val uniting = TreeMap.newBuilder[Int, String](using strict)
    uniting += 9 -> "i"
    uniting += 2 -> "B"
    uniting ++= src
    println(uniting.result())
    // Another map's entries go into the tree all at once: an ordering failing part way leaves the builder as it was.
    var failOn20 = false
    val partial: Ordering[Int] = new Ordering[Int]:
      def compare(x: Int, y: Int): Int =
        if failOn20 && (x == 20 || y == 20) then throw new IllegalStateException("twenty")
        Integer.compare(x, y)
    val srcP = TreeMap(1 -> "a", 2 -> "b", 3 -> "c")(using partial)
    val interrupted = TreeMap.newBuilder[Int, String](using partial)
    interrupted ++= srcP
    failOn20 = true
    attempt(interrupted ++= Map(10 -> "x", 20 -> "y"))
    failOn20 = false
    println(interrupted.result())
    // A map's own `foreachEntry` is what the builder walks it with (`adder.addForEach`).
    val own = new mutable.HashMap[Int, String]:
      override def foreachEntry[U](f: (Int, String) => U): Unit =
        println("own foreachEntry")
        super.foreachEntry(f)
    own += 7 -> "g"
    val fromOwn = TreeMap.newBuilder[Int, String]
    fromOwn ++= own
    println(fromOwn.result())
    val fromHash = TreeMap.newBuilder[Int, String]
    fromHash ++= Map(2 -> "b", 1 -> "a")
    fromHash ++= SortedMap(3 -> "c")(using Ordering.Int.reverse)
    println(fromHash.result())
    val reused = TreeMap.newBuilder[Int, String]
    reused += 5 -> "e"
    val first = reused.result()
    reused += 4 -> "d"
    println(s"$first ${reused.result()}")
    reused.clear()
    reused += 7 -> "g"
    println(reused.result())
    // A key the ordering calls equal, not the same reference, keeps the key and takes the value.
    val folded = TreeMap.newBuilder[String, Int](using Ordering.by[String, String](_.toLowerCase))
    folded ++= List("A" -> 1, "b" -> 2, "a" -> 3)
    println(folded.result())

    // An ordering that throws is met before a function that throws at the third entry.
    val bad: Ordering[Int] = new Ordering[Int]:
      def compare(a: Int, b: Int): Int = throw new IllegalStateException("ordering")
    val m = TreeMap(1 -> "a", 2 -> "b", 3 -> "c")
    def f(e: (Int, String)): (Int, String) = if e._1 == 3 then throw new IllegalArgumentException("third entry") else e
    attempt(m.map(f)(using bad))
    attempt(m.collect { case e => f(e) }(using bad))
    attempt(m.flatMap(e => List(f(e)))(using bad))
    attempt(m.map(e => (e._1 * 10, e._2)))
