
object Probe:
  def main(args: Array[String]): Unit = println(Array(1).pop())

// An array is asked to pop its last element.
// scalac 3.8.4 rejects it: value pop is not a member of Array[Int]
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: Probe$.main: no @jvm intrinsic or body for pop
// expect: 1 construct not supported by the JVM backend
