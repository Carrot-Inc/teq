def run(f: () => Int = () => return 1): Int = f()

object Probe:
  def main(args: Array[String]): Unit = println(run())

// A closure used as a default argument returns from its enclosing method.
// scalac 3.8.4 rejects it: method run$default$1 has a return statement; it needs a result type
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: return_in_default_lambda.scala:1:37: warning: Discarded non-Unit value of type Int. Add `: Unit` to discard silently.
// expect: jvm backend: return_in_default_lambda$package$.run: a non-local return of a def that makes no key
// expect: 1 construct not supported by the JVM backend
