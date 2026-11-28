object Values:
  def count$1(): Int = 1
  def read(): Int =
    def count(): Int = 2
    count()

object Probe:
  def main(args: Array[String]): Unit = println(Values.read())

// A local method and an explicitly declared member have the same name and descriptor after lifting.
// scalac 3.8.4 builds it. The JVM reports java.lang.ClassFormatError: Duplicate method name "count$1" with signature "()I" in class file Values$.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Values$.read: internal error: a second lifted method count$1()I in the class
// expect: 1 construct not supported by the JVM backend
