//> using platform js
//> using jsVersion 1.21.0
// interp-expected: js
// Reflective lookups of a parameterless given's module class, a given with a using clause's class and a class of a
// given: loadable, instantiatable, and the class's constructor's parameters (none: no outer instance).
import scala.scalajs.reflect.Reflect
import scala.scalajs.reflect.annotation.EnableReflectiveInstantiation
@EnableReflectiveInstantiation trait R
object Host {
 given G: R with { class K(val n: Int) extends R }
 given g(using n: Int): R with { class K(val x: Int) extends R }
}
object Main { def main(args: Array[String]): Unit = {
 println(Reflect.lookupLoadableModuleClass("Host$G$").isDefined)
 println(Reflect.lookupInstantiatableClass("Host$g").isDefined)
 println(Reflect.lookupInstantiatableClass("Host$G$K").get.declaredConstructors.head.parameterTypes.size)
} }
