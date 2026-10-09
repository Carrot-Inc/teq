// jars: fixtures
// targets: js interp jvm
// A jar body passes a using clause explicitly: `Step.Guard.apply[Int](Step.Guard.Proof.legal[Int](value, value))`,
// the given `apply` of an object that shares its name with a class nested beside it, and a given with
// parameters applied to the `NotGiven.value`s scalac synthesized, which are no givens themselves.
import fix.guard.{Step, Steps}

@main def main(): Unit =
  println(Steps().twice(21).run())
  println(Steps().twice(-1).run())
