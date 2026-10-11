// A `return` in a lambda leaves the def around it, on the JVM as dotty's `NonLocalReturns` writes it (master: "not
// supported on the JVM yet"): a fresh key per run of the def, captured by the closures through nested lambdas and
// thrown in a `NonLocalReturnControl` that the def's handler takes by its key; through a by-name argument, a
// `finally` that still runs, a local def that is the return's own target, a recursive def whose runs keep apart,
// a `Unit` def, a value class's and a trait's method, a generic and a `Double` result.
import scala.annotation.nowarn
@nowarn def first(xs: List[Int]): Int = { xs.foreach(x => if x > 1 then return x); -1 }
@nowarn def nested(xss: List[List[Int]]): String = { xss.foreach(xs => xs.foreach(x => if x == 3 then return s"found $x")); "none" }
@nowarn def byName(o: Option[Int]): Int = { val v = o.getOrElse(return 0); v * 10 }
var log = List.empty[String]
@nowarn def withFinally(xs: List[Int]): Int =
  try { xs.foreach(x => if x == 2 then return x * 100); 0 }
  finally log = "finally" :: log
@nowarn def local(xs: List[Int]): Int =
  def inner(): Int = { xs.foreach(x => if x > 0 then return x + 1000); -2 }
  inner() + 1
@nowarn def recursive(n: Int): Int = { List(n).foreach(x => if x > 0 then return recursive(x - 1) + 1); 0 }
@nowarn def unitRet(xs: List[Int]): Unit = { xs.foreach(x => if x == 1 then { log = "unit" :: log; return }); log = "after" :: log }
class V(val xs: List[Int]) extends AnyVal:
  @nowarn def firstBig: Int = { xs.foreach(x => if x > 10 then return x); 0 }
trait T:
  def items: List[String]
  @nowarn def find(p: String => Boolean): String = { items.foreach(s => if p(s) then return s); "?" }
class C extends T { def items = List("a", "bb", "ccc") }
object O:
  @nowarn def generic[A](xs: List[A], p: A => Boolean): Option[A] = { xs.foreach(x => if p(x) then return Some(x)); None }
  @nowarn def dbl(xs: List[Double]): Double = { xs.foreach(x => if x > 1.5 then return x + 0.25); 0.5 }
@main def run() =
  println(first(List(1, 2, 3)))
  println(first(List(0)))
  println(nested(List(List(1), List(2, 3))))
  println(byName(None))
  println(byName(Some(4)))
  println(withFinally(List(1, 2, 3)))
  println(log)
  println(local(List(5)))
  println(recursive(3))
  unitRet(List(1))
  println(log)
  println(new V(List(1, 20, 30)).firstBig)
  println(new C().find(_.length == 2))
  println(O.generic(List("x", "yy"), _.startsWith("y")))
  println(O.dbl(List(1.0, 2.0)))
