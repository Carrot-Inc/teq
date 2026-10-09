//> using platform js
//> using jsVersion 1.21.0
// interp-expected: js
// Reflective lookups of classes in the root package, where a class may be named like a lookup
// key the runtime could make up (`null`) or with an operator (`+`, looked up by its source name).
import scala.scalajs.reflect.Reflect
import scala.scalajs.reflect.annotation.EnableReflectiveInstantiation

@EnableReflectiveInstantiation
class `null`

@EnableReflectiveInstantiation
class +

@main def main(): Unit =
  println(Reflect.lookupInstantiatableClass(null).isDefined)
  println(Reflect.lookupInstantiatableClass("null").isDefined)
  println(Reflect.lookupInstantiatableClass("+").isDefined)
  println(Reflect.lookupInstantiatableClass("$plus").isDefined)
