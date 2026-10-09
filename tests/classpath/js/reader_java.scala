// jars: fixtures fixtures-rdecl javafix
// targets: interp
// A library body's call of an overload whose parameter is a Java class's static nested class,
// named in the signature under the class's companion (`fix.Outer$.Nested`), compiled when the
// name had that one alternative. The interpreter alone: the JavaScript target
// has no Java classes, and the JVM target emits the unread class as `fix/Outer/Nested`.
import fix.reader.*

@main def main(): Unit =
  println(RdCalls.javaNested)
