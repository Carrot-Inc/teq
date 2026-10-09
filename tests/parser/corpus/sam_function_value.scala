// A method named where a trait with a single abstract method is expected implements the trait
// even when its argument was typed before the alternative was chosen (`map(f)` among overloaded
// `map`s); a lambda literal fitting both a function type and a SAM type takes the function type
// (dotc's `SAMArgOK` in the specificity test); a value of function type converts to neither.
trait Sam[A, B]:
  def apply(a: A): B

trait Base:
  def map[T](f: Sam[Int, T]): String = "base " + f(21)
object Main extends Base:
  def map[T: scala.reflect.ClassTag](f: Sam[Int, T]): String = "tagged " + f(21)
  def run[T](s: Sam[Int, T]): T = s(21)
  def pick(f: String => String): Int = 1
  def pick(f: java.util.function.Function[String, String]): Int = 2
  def double(x: Int): Int = x * 2
  def named(x: Int): String = "n" + x
  def main(args: Array[String]): Unit =
    println(run(double))
    println(run(_ + 1))
    println(run(named))
    println(map(double))
    println(pick(x => x))
    println(pick((s: String) => s.trim))
