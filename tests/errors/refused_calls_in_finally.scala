class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("bad_z") def first(): Int = 1
@jvm("bad_a") def second(): Int = 2
object Probe { def f(b: Boolean): Int = try { if b then return 3; 4 } finally { println(first()+second()) }; def main(args: Array[String]): Unit = println(f(true)) }

// A return copies the finalizer containing both unsupported instructions.
// scalac 3.8.4 builds it and prints `3` then `3`.
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Probe$.f: the instruction bad_a of bad_a
// expect: jvm backend: Probe$.f: the instruction bad_z of bad_z
// expect: 2 constructs not supported by the JVM backend
