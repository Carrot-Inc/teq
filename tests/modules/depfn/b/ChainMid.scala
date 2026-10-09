package dfb

import dfa.DepFn

// The signature exactly: a function of that type passes, and a lambda's parameter is of it.
object ChainMid:
  def message: String = DepFn.inspect(List("chain"))(xs => summon[String] + xs.size)
  val exact: (ctx: String) ?=> List[Option[ctx.type]] => Int = xs => xs.size
  def passed: Int = DepFn.inspect(List("p"))(exact)
  def precise: Int = DepFn.inspect(List("q"))((ctx: String) ?=> xs => Same[List[Option[ctx.type]]]()(xs).size)

final class Same[B]:
  def apply[A](a: A)(using A =:= B): A = a
