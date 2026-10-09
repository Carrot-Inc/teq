// scala-library's `util.control.TailCalls`, as zio's `FiberId.toSetLazy` uses it: `done`,
// `tailcall`, `map`, `flatMap` and `result`, deep enough that a naive recursion would overflow.
import scala.util.control.TailCalls.*

object Main:
  def isEven(n: Int): TailRec[Boolean] = if n == 0 then done(true) else tailcall(isOdd(n - 1))
  def isOdd(n: Int): TailRec[Boolean] = if n == 0 then done(false) else tailcall(isEven(n - 1))

  def sum(xs: List[Int]): TailRec[Int] = xs match
    case Nil => done(0)
    case h :: t => tailcall(sum(t)).map(_ + h)

  def fib(n: Int): TailRec[Int] =
    if n < 2 then done(n)
    else
      for
        a <- tailcall(fib(n - 1))
        b <- tailcall(fib(n - 2))
      yield a + b

  def main(args: Array[String]): Unit =
    println(isEven(100001).result)
    println(sum(List.range(1, 20001)).result)
    println(fib(15).result)
