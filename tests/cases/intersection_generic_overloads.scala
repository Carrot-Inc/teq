// Overloads of one name joined across an intersection whose parts are applied in a body: the
// merged overload set compares the alternatives' parameters seen from each part's arguments,
// and with the parallel typer's overlays those arguments are a worker's types, canonicalised into
// the base under the loader's lock before the comparison.
trait Reader[T]:
  def body(t: T): String = s"reader $t"

trait Writer[T]:
  def body(t: T, times: Int): String = s"writer ${t.toString * times}"

trait Both[A, B] extends Reader[A], Writer[B]

object Main:
  def main(args: Array[String]): Unit =
    val x: Reader[Int] & Writer[String] = new Reader[Int] with Writer[String] {}
    println(x.body(1))
    println(x.body("ab", 2))
    val y: Reader[List[Int]] & Writer[Option[Char]] = new Reader[List[Int]] with Writer[Option[Char]] {}
    println(y.body(List(1, 2)))
    println(y.body(Some('c'), 3))
    val z: Both[Double, Boolean] = new Both[Double, Boolean] {}
    println(z.body(2.5))
    println(z.body(true, 1))
    wildcardBound()

  // A part applied to a wildcard whose parameter's bound is a local class: the bound is read for
  // the member's parameters seen from the wildcard, in the holder's namespace under the lock.
  def wildcardBound(): Unit =
    class Bound
    trait BoundedReader[T <: Bound]:
      def body(t: T): String = "bounded reader"
    val w: BoundedReader[?] & Writer[Int] = new BoundedReader[Bound] with Writer[Int] {}
    println(w.body(1, 2))
