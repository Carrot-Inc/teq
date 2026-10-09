import scala.annotation.tailrec

// `@tailrec` methods whose recursive call is the root of a plain inline call's expansion (an
// inline parameter, a by-name one, none): the check counts the calls the body keeps, not the
// expansion's root the later phase moved into the call's node.
object M:
  inline def next(inline n: Int): Int = A.loop(n)
  inline def nextByName(n: => Int): Int = B.loop(n)
  inline def again: Int = B.loop(0)
object A:
  @tailrec def loop(n: Int): Int =
    if n <= 0 then 42 else M.next(n - 1)
object B:
  @tailrec def loop(n: Int): Int =
    if n <= 0 then 7 else if n == 1 then M.again else M.nextByName(n - 1)
@main def main(): Unit =
  println(A.loop(10))
  println(B.loop(10))
