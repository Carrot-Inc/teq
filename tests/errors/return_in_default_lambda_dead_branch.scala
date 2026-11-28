def run(f: () => Int = () => {
  if false then
    return 1
  else 2
}): Int = f()

object Probe:
  def main(args: Array[String]): Unit =
    println(run())

// A closure used as a default argument returns from its enclosing method. It appears in the untaken
// branch of a constant conditional.
// scalac 3.8.4 rejects it: method run$default$1 has a return statement; it needs a result type
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: return_in_default_lambda_dead_branch.scala:3:12: warning: Discarded non-Unit value of type Int. Add `: Unit` to discard silently.
// expect: jvm backend: return_in_default_lambda_dead_branch$package$.run: a non-local return of a def that makes no key
// expect: 1 construct not supported by the JVM backend
