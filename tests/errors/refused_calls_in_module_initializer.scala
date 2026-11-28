class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("bad_z") def first(): Int = 1
@jvm("bad_a") def second(): Int = 2
object Probe { val a = first(); println(second()); def main(args: Array[String]): Unit = println(a+first()) }

// The module initializer and main both call methods with unsupported instructions.
// scalac 3.8.4 builds it and prints `2` then `2`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Probe$.main: the instruction bad_z of bad_z
// expect: jvm backend: Probe$: the instruction bad_a of bad_a
// expect: jvm backend: Probe$: the instruction bad_z of bad_z
// expect: 3 constructs not supported by the JVM backend
