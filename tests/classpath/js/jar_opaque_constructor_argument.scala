// jars: scala-library defaults-lib
// targets: js interp
// An opaque type of two parameters passed as a higher-kinded type argument in a body of a jar
// (tests/support/defaults_lib.scala's `PosWrapOps`) and from source: the unapplied constructor is within the
// parameter's bound `[_, _] =>> Any` in both.
import defaultslib.*

@main def main(): Unit =
  println(PosWrapOps.shown)
  println(WrapMin.of[Wrapped, String](WrapType.wrapped, "a"))
