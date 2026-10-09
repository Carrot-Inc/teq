// jars: scala-library
// scala.util.control.TailCalls (a sealed trampoline with a tail-recursive interpreter) and
// Breaks (a ControlThrowable caught by breakable) are compiled from their TASTy bodies.
import scala.util.control.TailCalls.*
import scala.util.control.Breaks.*
object Main:
  def isEven(n: Int): TailRec[Boolean] = if n == 0 then done(true) else tailcall(isOdd(n - 1))
  def isOdd(n: Int): TailRec[Boolean] = if n == 0 then done(false) else tailcall(isEven(n - 1))
  def fib(n: Int): TailRec[Int] =
    if n < 2 then done(n)
    else for
      a <- tailcall(fib(n - 1))
      b <- tailcall(fib(n - 2))
    yield a + b
  def main(args: Array[String]): Unit =
    println(isEven(100001).result)
    println(fib(20).result)
    println(done(3).map(_ + 1).result)
    var sum = 0
    breakable {
      for i <- 1 to 100 do
        if i > 10 then break()
        sum += i
    }
    println(sum)
    val found = tryBreakable {
      for i <- List(1, 2, 3, 4) do if i == 3 then break()
      "no"
    } catchBreak { "broke" }
    println(found)
