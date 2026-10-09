// A conversion method from a Java supertype is the implicit function value of a subtype's function
// type, by contravariance (scalac: "2", "found").
import scala.language.implicitConversions
implicit def size(l: java.util.List[Int]): Int = l.size
def f(implicit c: java.util.ArrayList[Int] => Int): Int =
  val xs = new java.util.ArrayList[Int]()
  xs.add(1)
  xs.add(2)
  c(xs)
def g(implicit c: java.util.ArrayList[Int] => Int): String = "found"
@main def main(): Unit =
  println(f)
  println(g)
