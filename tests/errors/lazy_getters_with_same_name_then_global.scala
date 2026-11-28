class JSGlobal(name: String) extends scala.annotation.StaticAnnotation
object js:
  def native: String = "sep"
object Paths:
  @JSGlobal("sep") val separator: String = js.native

class Count:
  lazy val `!`: Int = 1
  lazy val $bang: Int = 2

object Probe:
  def main(args: Array[String]): Unit =
    println(new Count().$bang)
    println(Paths.separator)

// Two lazy members have names that encode to the same JVM getter name. Next, a value annotated as a
// JavaScript global is read.
// scalac 3.8.4 builds it. The JVM reports java.lang.ClassFormatError: Duplicate field name "$bang$lzy1$lzyHandle" with signature "Ljava.lang.invoke.VarHandle;" in class file Count.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Count: internal error: a second lazy initialiser $bang$lzyINIT1 in the class
// expect: jvm backend: Count: internal error: a second lazy val's getter $bang()I in the class
// expect: jvm backend: Probe$.main: JavaScript interop
// expect: 3 constructs not supported by the JVM backend
