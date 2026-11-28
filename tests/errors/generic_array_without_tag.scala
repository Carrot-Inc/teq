def allocate[T](zero: T): AnyRef = scala.newArray[T](1, zero)

object Probe:
  def main(args: Array[String]): Unit = println(allocate(1) != null)

// A generic method calls scala.newArray for its type parameter.
// scalac 3.8.4 rejects it: value newArray is not a member of scala - did you mean scala.Array?
// command: build
// teq: --target jvm --all-mains
// jars: scala-library
// exit: 3
// expect: jvm backend: generic_array_without_tag$package$.allocate: a new array of an element type only a ClassTag knows
// expect: 1 construct not supported by the JVM backend
