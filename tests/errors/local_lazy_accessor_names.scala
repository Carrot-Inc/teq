def read(): Int =
  lazy val `!`: Int = 1
  lazy val $bang: Int = 2
  0 + `!` + $bang

object Probe:
  def main(args: Array[String]): Unit =
    println(read())

// Two local lazy vals have names that encode to the same JVM accessor name.
// scalac 3.8.4 builds it. The JVM reports java.lang.ClassFormatError: Duplicate method name "$bang$lzyINIT1$1" with signature "(Lscala.runtime.LazyInt;)I" in class file local_lazy_accessor_names$package$.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: local_lazy_accessor_names$package$.read: internal error: a second local lazy val's method $bang$1(Lscala/runtime/LazyInt;)I in the class
// expect: jvm backend: local_lazy_accessor_names$package$.read: internal error: a second local lazy val's method $bang$lzyINIT1$1(Lscala/runtime/LazyInt;)I in the class
// expect: jvm backend: local_lazy_accessor_names$package$.read: internal error: two lifted methods named $bang$1(Lscala/runtime/LazyInt;)I in the class
// expect: jvm backend: local_lazy_accessor_names$package$.read: internal error: two lifted methods named $bang$lzyINIT1$1(Lscala/runtime/LazyInt;)I in the class
// expect: 4 constructs not supported by the JVM backend
