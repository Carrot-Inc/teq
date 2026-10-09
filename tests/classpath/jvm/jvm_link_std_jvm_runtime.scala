// jars: scala-library
// std: scala-library
// targets: jvm
// The JVM target's linking: scala-library's bytecode as the JVM std.
// std/jvm.scala is written against the lean std: enum valueOf calls the lean `illegalArgument`,
// a sequence pattern with a rest builds the lean `ArraySeq(ArrayList)`; both fail to type under
// --std=scala-library.
enum Color:
  case Red, Green
@main def run(): Unit =
  println(Color.valueOf("Green"))
  Array(1, 2, 3) match
    case Array(a, rest*) => println(s"$a ${rest.toList}")
