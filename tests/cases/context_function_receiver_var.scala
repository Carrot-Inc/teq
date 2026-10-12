// An operator on a receiver whose type is still an open variable that something bounds below:
// dotty's selection instantiates it from below and tries again (`Typer.tryInstantiateTypeVar`,
// `couldInstantiateTypeVar`), so `it + 1` inside `fun(..)`'s context function takes `T`'s `Int`
// from the expected `Int => ?` (scala3's i10082).
object Kotlin:
  class Ctx[T](val x: T) extends AnyVal
  def fun[T, U](fn: Ctx[T] ?=> U): T => U = (x: T) => fn(using Ctx(x))
  def it[T](using ctx: Ctx[T]) = ctx.x

import Kotlin.*

@main def main(): Unit =
  println(List(1).map(fun(it + 1)))
  println(List(2.5).map(fun(it * 3)))
  println(List("a").map(fun(it + "!")))
