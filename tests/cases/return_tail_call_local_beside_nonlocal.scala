// teq: --wconf msg=Non.local.returns.are.no.longer.supported:s
// A local def and the def around it each loop by a `return` under an operand: each its own loop, each `return` its
// own def's (on JavaScript each a call, which overflows the stack: a pending row); a lambda's `return` beside them
// stays the method's non-local return. A hundred thousand deep.
object Nest {
  final def twoDefs(n: Int): Int = {
    def local(k: Int): Int = 1 + (if k > 0 then return local(k - 1) else 0)
    (if n > 0 then return twoDefs(n - 1) else local(100000)) + 30
  }
  final def nonLocalAndThrown(n: Int): Int = {
    List(1).foreach { _ => if n == 7 then return 777 }
    1 + (if n > 0 then return nonLocalAndThrown(n - 1) else 0)
  }
}
object Main {
  def main(args: Array[String]): Unit = {
    println(Nest.twoDefs(100000))
    println(Nest.nonLocalAndThrown(10))
    println(Nest.nonLocalAndThrown(100000))
  }
}
