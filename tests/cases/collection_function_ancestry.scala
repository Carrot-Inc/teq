// scala-library's collections are functions: `collection.Map[K, V]` a `PartialFunction[K, V]` (`MapOps`),
// `collection.Seq[A]` a `PartialFunction[Int, A]`, `collection.Set[A]` an `A => Boolean`. A collection keeps
// its object representation outside the JVM and is adapted to a function where it meets one: an expected
// function type, a member a function type declares (`lift`, `andThen`, `orElse`), a type test and a cast. A
// program's class that is a function through its own trait is one as it stands.
import scala.collection.mutable

trait Doubler extends (Int => Int)
class Twice extends Doubler:
  def apply(x: Int): Int = 2 * x

object Main:
  def applyAll(f: Int => String, xs: List[Int]): List[String] = xs.map(f)
  def main(args: Array[String]): Unit =
    val m: mutable.Map[Int, String] = mutable.Map(1 -> "one", 2 -> "two")
    val p: PartialFunction[Int, String] = m
    println(s"${p.isDefinedAt(1)} ${p.isDefinedAt(3)}")
    println(List(1).map(p))
    println(applyAll(Map(1 -> "a", 2 -> "b"), List(2, 1)))
    println(List(1, 3).collect(m))
    println(s"${m.lift(2)} ${m.lift(5)}")
    println((m.andThen(_.length))(2))
    println((m.orElse { case _ => "other" })(9))
    val s = Set(1, 2)
    println(List(1, 2, 3).filter(s))
    val xs = Vector("x", "y")
    println(List(0, 1).map(xs))
    println(xs.lift(4))
    val any: Any = m
    println(s"${any.isInstanceOf[PartialFunction[?, ?]]} ${any.isInstanceOf[Function1[?, ?]]}")
    val f = any.asInstanceOf[Int => String]
    println(f(1))
    any match
      case pf: PartialFunction[Int, String] @unchecked => println(pf.applyOrElse(7, (_: Int) => "none"))
      case _ => println("no")
    val d: Doubler = new Twice
    println(s"${d(21)} ${List(1, 2).map(d)} ${(d.andThen(_ + 1))(5)}")
