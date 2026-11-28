class native extends scala.annotation.StaticAnnotation
@native trait Counter:
  def count(): Int
def read(c: Counter): Int = c.count()

object Probe:
  def main(args: Array[String]): Unit = println(1)

// The method calls a member of a trait marked with the program's native annotation.
// scalac 3.8.4 builds it and prints `1`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: native_member_call$package$.read: a member of a JavaScript type
// expect: 1 construct not supported by the JVM backend
