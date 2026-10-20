// lila: Page.js(esmInit("x") ++ cond.so(Esm("mod.user")) ++ cond2.so(Esm("y")))
// `so[A](a: => A)(using Zero[A]): A` is typed against IterableOnce[?B]; its argument Esm
// only fits through the Conversion[Esm, List[Esm]], which makes List[Esm] the lower bound of
// A. scalac instantiates A to that converted type (the only given is Zero[List[X]], and Zero is
// invariant). The second `so` (Zero[Esm] in teq) is the cascade seen at page.scala:49/80.
import scala.language.implicitConversions
trait Zero[A] { def zero: A }
given [X]: Zero[List[X]] with { def zero = Nil }
case class Esm(key: String)
given Conversion[Esm, List[Esm]] = e => List(e)
extension (self: Boolean) def so[A](a: => A)(using zero: Zero[A]): A = if self then a else zero.zero

@main def run() =
  val modules: List[Esm] = List(Esm("a")) ++ true.so(Esm("b")) ++ false.so(Esm("c"))
  println(modules)
