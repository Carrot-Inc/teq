// jars: depfun-lib
// targets: js interp jvm
// Function types whose result names a parameter, read from a jar's TASTy (tests/support/
// depfun_lib.scala): a call's result has the argument's path (`IntCtx.T`, `left.T`), a lambda
// passed to one sees its own parameter in the result, and a contextual one takes its argument
// from a `using` clause or from the given in scope.
import depfunlib.*
@main def run(): Unit =
  val n: Int = Fns.plain(IntCtx)
  val k: Int = Fns.contextual(using IntCtx)(List(1, 2))
  println(n + k)
  val left: Ctx = StrCtx
  val l: left.T = Fns.plain(left)
  println(left.show(l))
  println(Fns.showing(IntCtx)(4))
  println(Fns.run(c => c.make))
  println(Fns.withCtx(xs => xs.size))
  given StrCtx.type = StrCtx
  val s: String = Fns.contextual(List("a")).toString + Fns.made
  println(s)
