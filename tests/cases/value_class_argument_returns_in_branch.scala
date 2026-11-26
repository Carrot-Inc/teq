// A value class's instance made over an argument that returns, as a branch of an `if` whose value is wanted: the
// instance is its underlying value, so the branch is the `return` itself (scalac's erasure drops the constructor),
// and the method returns from it with nothing written after the return.
final class V(val u: Int) extends AnyVal

object Main {
  def returns(x: Int): Int = { val m = if (x > 0) new V(return 7) else new V(2); m.u }
  def blockReturns(x: Int): Int = { val m = if (x > 0) new V({ println("b"); return 7 }) else new V(2); m.u }
  def main(args: Array[String]): Unit = {
    println(returns(1))
    println(returns(-1))
    println(blockReturns(1))
    println(blockReturns(-1))
  }
}
