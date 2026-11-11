// A `@tailrec` method of several parameter clauses: its recursive call is one call, in tail position, whichever
// clause is a `using` one.
import scala.annotation.tailrec

@tailrec
def count(label: String)(n: Int, acc: Int): Int =
  if n == 0 then acc else count(label)(n - 1, acc + 1)

@tailrec
def sum(n: Int, acc: Int)(using step: Int): Int =
  if n <= 0 then acc else sum(n - step, acc + n)

object Walk:
  @tailrec
  def down(using floor: Int)(n: Int)(acc: List[Int]): List[Int] =
    if n < floor then acc else down(n - 1)(n :: acc)

@main def run(): Unit =
  println(count("c")(3, 0))
  given Int = 2
  println(sum(9, 0))
  println(Walk.down(using 3)(6)(Nil))
