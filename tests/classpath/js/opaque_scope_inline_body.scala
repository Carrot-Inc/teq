// jars: fixtures
// targets: js interp jvm
// An opaque type of a jar is transparent in its scope's bodies, as a source one is in its
// file: `Step.apply`, an inline method of the opaque type's companion, returns the function the
// opaque type stands for, and its body expands at the program's call.
import fix.guard.{Step, Steps}

@main def main(): Unit =
  println(Steps().twice(21).run())
  println(Step("direct").run())
