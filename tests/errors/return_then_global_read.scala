class JSGlobal(name: String) extends scala.annotation.StaticAnnotation
object js:
  def native: String = "sep"
object Paths:
  @JSGlobal("sep") val separator: String = js.native

def run(f: () => Int = () => {
  return 1
  println(Paths.separator)
  2
}): Int = f()

object Probe:
  def main(args: Array[String]): Unit =
    println(run())

// A closure used as a default argument returns from its enclosing method. Next, a value annotated as
// a JavaScript global is read.
// scalac 3.8.4 rejects it: method run$default$1 has a return statement; it needs a result type
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: return_then_global_read.scala:8:10: warning: Discarded non-Unit value of type Int. Add `: Unit` to discard silently.
// expect: jvm backend: return_then_global_read$package$.run: JavaScript interop
// expect: jvm backend: return_then_global_read$package$.run: a non-local return of a def that makes no key
// expect: 2 constructs not supported by the JVM backend
