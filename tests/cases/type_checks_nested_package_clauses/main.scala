package a
package b
import scala.compiletime.testing.typeChecks
@main def run(): Unit =
  println(1)
  Predef.println(typeChecks("println(\"s\")"))
  Predef.println(typeChecks("println(2)"))
