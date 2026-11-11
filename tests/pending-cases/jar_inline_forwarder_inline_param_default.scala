// jars: scala-library defaults-lib
// targets: js interp
// A jar's forwarders made through a companion extension method (tests/support/defaults_lib.scala's `InlExports`), called
// leaving arguments out: `k`'s `a` takes the getter of its name, `k$default$2(s)` (`b`'s), and `h`'s inline `a` the plain
// getter `h$default$2(s)`, as dotty's `tryDefault` finds them by name (Applications.scala:382-402, Namer.scala:1392-1398).
// The JVM build calls the jar's getters and prints scalac's 1279 and 247
// (tests/classpath/jvm/jar_inline_extension_forwarders.scala). The interpreter takes a default from its callee's function
// or its folded parameter default, which a forwarder whose getters are numbered for its member has none of (interp/eval.rs
// `default_apply`, `default_target`): "has no default for its parameter 2" and "has no default getters". The JavaScript
// build calls getters the link does not keep. Moves to tests/classpath/js with the change that runs the jar's getter
// methods, as the JVM runs them.
import defaultslib.*

@main def main(): Unit =
  println(InlExports.k("x")(b = 7, c = 9))
  println(InlExports.h("xy")(b = 7))
