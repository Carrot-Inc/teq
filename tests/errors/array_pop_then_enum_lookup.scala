class jvm(template: String) extends scala.annotation.StaticAnnotation
@jvm("$enumValues($0)") def values(): Int = 6

object Probe:
  def main(args: Array[String]): Unit =
    println(Array(1).pop())
    println(values())

// An array pop precedes an enum lookup in the same method.
// scalac 3.8.4 rejects it: value pop is not a member of Array[Int]
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Probe$.main: an enum lookup whose companion is not known
// expect: jvm backend: Probe$.main: no @jvm intrinsic or body for pop
// expect: 2 constructs not supported by the JVM backend
