// jars: scala-library defaults-lib
// targets: jvm interp
// The arguments of a jar's inline forwarder made through a companion
// extension method (tests/support/defaults_lib.scala's `InlExports.h`, of
// `inline def h(inline a: Int = 3, b: Int = 4)`), evaluated in scalac 3.8.4's
// order: `b`, then the receiver `s`, then the inline `a` where the body
// reads it. The zip of the parameters' flags (Namer.scala:1392-1398)
// gives the receiver `a`'s `Inline` while the formals keep the member's
// (`a: Int @InlineParam`). Reviewer #9's [R2] of the workers exports helper.
// Moves to tests/classpath/jvm with the change that binds them in scalac's order.
import defaultslib.*

def eff[A](tag: String, a: A): A = { println(s"eval $tag"); a }

@main def main(): Unit =
  println(InlExports.h(eff("s", "xy"))(eff("a", 6), eff("b", 7)))
