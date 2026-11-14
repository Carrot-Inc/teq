// A class that joins a method the standard library declares with a trait's method of the same name and other
// parameters changes nothing for the rest of the program: partial functions, sequence patterns, an exception's
// cause, `indexOf`, a map's `-`, `compareTo` and plain function values answer as scalac 3.8.4 has them.
trait Tester { def isDefinedAt(x: String, y: Int): Boolean = true; def applyOrElse(x: String): Int = 0 }
abstract class Guard extends PartialFunction[Int, Int] with Tester

trait Counted { def iterator(step: Int): Int = step; def drop(a: String): Int = 0; def hasNext(n: Int): Boolean = false; def next(n: Int): Int = n }
abstract class Shelf extends Iterable[Int] with Counted
abstract class Walk extends Iterator[Int] with Counted

trait Caused { def getCause(n: Int): Throwable = null }
abstract class Failed extends RuntimeException with Caused

trait Indexed { def indexOf(s: String): Int = 42 }
abstract class Row extends scala.collection.Seq[Int] with Indexed

trait Flagged { def indexOf(b: Boolean): Int = 99 }
abstract class Ops extends scala.collection.SeqOps[Int, List, List[Int]] with Flagged

trait Minus { def -(s: String): Int = s.length }
abstract class Table extends scala.collection.immutable.Map[Int, Int] with Minus

trait Ranked { def compareTo(s: Int, t: Int): Int = s - t }
abstract class Graded extends Comparable[String] with Ranked

class Twice extends (Int => Int) with ((Int, Int) => Int) {
  def apply(x: Int): Int = x + 10
  def apply(x: Int, y: Int): Int = x + y
}

@main def run(): Unit =
  val pf: PartialFunction[Int, String] = { case 1 => "one" }
  println(pf.isDefinedAt(1))
  println(pf.isDefinedAt(2))
  println(pf.applyOrElse(2, (_: Int) => "other"))
  println(List(1, 2, 1).collect(pf))
  val total: Int => String = pf
  println(List(1, 1).map(total))
  println(pf.lift(2))
  List(1, 2, 3) match
    case Seq(a, b, rest*) => println(s"$a $b $rest")
    case _ => println("no")
  println(Array(3, 1, 2).sorted.toList)
  println(Set(1, 2).hashCode == Set(2, 1).hashCode)
  println(List(1, 2).hashCode == List(1, 2).hashCode)
  val e = new Exception("outer")
  e.initCause(new Exception("inner"))
  println(e.getCause == null)
  println(e.getCause.getMessage)
  println(List(1, 2, 3).indexOf(2))
  println(List(1, 2, 3).indexOf(2, 0))
  val xs: scala.collection.SeqOps[Int, List, List[Int]] = List(1, 2, 3)
  println(xs.indexOf(2))
  println(xs.indexOf(2, 2))
  val m = Map(1 -> 10, 2 -> 20)
  println(m - 1)
  val c: Comparable[String] = "a"
  println(c.compareTo("b"))
  println(java.nio.charset.StandardCharsets.UTF_8.compareTo(java.nio.charset.StandardCharsets.UTF_8))
  val f: Int => Int = _ + 1
  println(f(1))
  val g: (Int, Int) => Int = _ * _
  println(g(3, 4))
  println(List(1, 2).map(f))
