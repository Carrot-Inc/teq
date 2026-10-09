// The retained body of an inline override evaluates its inline match's scrutinee once, as
// scalac's reduction binds a scrutinee that is not pure: an effect before the value, a by-name
// parameter's argument, and a case that binds the scrutinee, which reads it once.
trait Base:
  def f(x: Int): Int
  def g(x: => Int): Int
  def h(x: Int): Int

object Ret extends Base:
  override inline def f(x: Int): Int = inline { println("selector"); x } match
    case _: Int => 42
  override inline def g(x: => Int): Int = inline x match
    case _: Int => 43
  override inline def h(x: Int): Int = inline { println("bound"); x } match
    case y: Int => y + 1

object RetainedEffects:
  def main(args: Array[String]): Unit =
    val b: Base = Ret
    println(b.f(1))
    println(b.g({ println("argument"); 1 }))
    println(b.h(1))
