// An application whose result is an intersection of its type arguments, against an expected
// intersection (`ZEnvironment(a, b)` for a `ZEnvironment[Base & Meta]`): either part could
// conform to each part of the expected type, so the expected type fixes neither, as dotc's
// result constraint keeps only what both ways imply, and the arguments decide.
final class Env[+R](val items: List[Any]):
  def size: Int = items.size
object Env:
  def apply[A, B](a: A, b: B): Env[A & B] = new Env(List(a, b))
  def one[A](a: A): Env[A] = new Env(List(a))

trait Base:
  def base: String = "base"
trait Meta:
  def meta: String = "meta"
final case class B() extends Base
final case class M() extends Meta

@main def main(): Unit =
  val env: Env[Base & Meta] = Env(B(), M())
  println(env.size)
  val swapped: Env[Base & Meta] = Env(M(), B())
  println(swapped.items)
  val single: Env[Base] = Env.one(B())
  println(single.items)
