class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("bad_z") def first(): Int = 1
@jvm("bad_a") def second(): Int = 2
object Probe { def main(args: Array[String]): Unit = println(first()+second()) }

// The two unsupported instructions occur in reverse lexical order.
// scalac 3.8.4 builds it and prints `3`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Probe$.main: the instruction bad_a of bad_a
// expect: jvm backend: Probe$.main: the instruction bad_z of bad_z
// expect: 2 constructs not supported by the JVM backend
