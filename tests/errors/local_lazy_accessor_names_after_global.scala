class JSGlobal(name: String) extends scala.annotation.StaticAnnotation
object js:
  def native: String = "sep"
object Paths:
  @JSGlobal("sep") val separator: String = js.native

def read(): Int =
  lazy val `!`: Int = 1
  lazy val $bang: Int = 2
  0 + `!` + $bang

object Probe:
  def main(args: Array[String]): Unit =
    println(Paths.separator)
    println(read())

// A value annotated as a JavaScript global is read. Next, two local lazy vals have names that encode
// to the same JVM accessor name.
// scalac 3.8.4 builds it. The run prints `sep`, then the JVM reports java.lang.ClassFormatError: Duplicate method name "$bang$lzyINIT1$1" with signature "(Lscala.runtime.LazyInt;)I" in class file local_lazy_accessor_names_after_global$package$.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Probe$.main: JavaScript interop
// expect: jvm backend: local_lazy_accessor_names_after_global$package$.read: internal error: a second local lazy val's method $bang$1(Lscala/runtime/LazyInt;)I in the class
// expect: jvm backend: local_lazy_accessor_names_after_global$package$.read: internal error: a second local lazy val's method $bang$lzyINIT1$1(Lscala/runtime/LazyInt;)I in the class
// expect: jvm backend: local_lazy_accessor_names_after_global$package$.read: internal error: two lifted methods named $bang$1(Lscala/runtime/LazyInt;)I in the class
// expect: jvm backend: local_lazy_accessor_names_after_global$package$.read: internal error: two lifted methods named $bang$lzyINIT1$1(Lscala/runtime/LazyInt;)I in the class
// expect: 5 constructs not supported by the JVM backend
