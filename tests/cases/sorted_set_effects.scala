// scala-library's sorted sets add each result of `map`, `flatMap`, `collect` and `zip` to the result's builder as it
// is made (`StrictOptimizedSortedSetOps`'s `strictOptimizedMap` .. `strictOptimizedZip`), so the result's ordering runs
// at the second element's insertion, before the third element is produced: an ordering that throws is met before a
// function that throws at the third element, on the immutable and the mutable sets alike.
import scala.collection.immutable.TreeSet
import scala.collection.mutable

object Main:
  def attempt(r: => Any): Unit =
    try println(r)
    catch case e: Exception => println(e.getMessage)
  def main(args: Array[String]): Unit =
    val s = TreeSet(1, 2, 3)
    val m = mutable.TreeSet(1, 2, 3)
    val bad: Ordering[Int] = new Ordering[Int]:
      def compare(a: Int, b: Int): Int = throw new IllegalStateException("ordering")
    val pf: PartialFunction[Int, Int] = { case x =>
      if x == 3 then throw new IllegalArgumentException("third element")
      x
    }
    def f(x: Int): Int = pf(x)
    attempt(s.collect(pf)(using bad))
    attempt(m.collect(pf)(using bad))
    attempt(s.map(f)(using bad))
    attempt(m.map(f)(using bad))
    attempt(s.flatMap(x => List(f(x)))(using bad))
    attempt(m.flatMap(x => List(f(x)))(using bad))
    val pairBad: Ordering[(Int, Int)] = new Ordering[(Int, Int)]:
      def compare(a: (Int, Int), b: (Int, Int)): Int = throw new IllegalStateException("ordering")
    def values: Iterator[Int] = Iterator(1, 2, 3).map(x => if x == 3 then throw new IllegalArgumentException("third element") else x)
    attempt(s.zip(values)(using pairBad))
    attempt(m.zip(values)(using pairBad))
    // A well-behaved ordering: the results as before.
    attempt(s.collect { case x if x > 1 => x * 10 })
    attempt(m.zip(List(9, 8, 7)))
    // The builder used again after a result keeps what it had (scala-library's `TreeSet` builder).
    val sb = TreeSet.newBuilder[Int]
    sb += 5
    val first = sb.result()
    sb += 4
    println(s"$first ${sb.result()}")
    // An ordering that calls two results equal keeps the first; through the static type Set the result is a plain set.
    val byHalf: Ordering[Int] = Ordering.by(_ / 2)
    println(s"${s.map(x => x)(using byHalf)} ${m.collect { case x => x }(using byHalf)}")
    val st: Set[Int] = s
    println(st.map(_ * 2).toList.sorted)
