// A method the standard library declares, joined by a class with a trait's method of the same name, is called
// by the library's own code under its own name: a comparison through `Comparable`, a sort by a `Comparator`, a
// `Runnable` run, a resource closed, and a class that is a library function and a trait's `apply` at once, called
// through both types: the lines scalac 3.8.4 prints.
trait Ranked { def compareTo(x: Int, y: Int): Int = x - y }
case class Ver(n: Int) extends Comparable[Ver] { def compareTo(o: Ver): Int = n - o.n }
abstract class Both extends Comparable[Both] with Ranked

trait Job { def run(n: Int): Int = n; def close(now: Boolean): Unit = (); def compare(x: String): Int = x.length; def getMessage(n: Int): String = "m" }
abstract class Task extends Runnable with AutoCloseable with Job
abstract class ByLen extends java.util.Comparator[String] with Job
class Boom extends RuntimeException("boom") with Job

trait Measure extends (String => Int) { def apply(x: String): Int = 1 }
trait Chosen { def apply(x: Option[Int]): Int = 2 }
class Scale extends Chosen with Measure

@main def run(): Unit =
  println(Ver(1).compareTo(Ver(2)))
  val c: Comparable[Ver] = Ver(5)
  println(c.compareTo(Ver(2)))
  println(List(Ver(3), Ver(1)).sorted(using Ordering.fromLessThan[Ver](_.compareTo(_) < 0)))
  val r = new Runnable { def run(): Unit = println("ran") }
  r.run()
  val boxed: Array[Integer] = Array(3, 1, 2)
  java.util.Arrays.sort(boxed, new java.util.Comparator[Integer] { def compare(a: Integer, b: Integer): Int = b - a })
  println(boxed.toList)
  try throw new IllegalStateException("bad") catch { case e: Exception => println(e.getMessage) }
  val res = new AutoCloseable { def close(): Unit = println("closed") }
  try println("using") finally res.close()
  val f: String => Int = new Scale
  println(f("x"))
  val o: Chosen = new Scale
  println(o.apply(None))
