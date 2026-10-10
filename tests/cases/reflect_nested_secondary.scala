//> using platform js
//> using jsVersion 1.21.0
// interp-expected: js
// Scala.js's reflective instantiation of a class nested in a class through its secondary
// constructor: the enclosing instance comes first, as for the primary one.
package reflectnested

import scala.scalajs.reflect.Reflect
import scala.scalajs.reflect.annotation.EnableReflectiveInstantiation

@EnableReflectiveInstantiation
trait Plugin:
  def name: String

class Outer(val n: Int):
  class In(val x: Int) extends Plugin:
    def this(s: String) = this(s.length)
    def name = s"in ${n + x}"

@main def run(): Unit =
  val outer = Outer(10)
  val ctors = Reflect.lookupInstantiatableClass("reflectnested.Outer$In").get.declaredConstructors
  for c <- ctors.sortBy(_.parameterTypes(1).getName) do
    val arg: Any = if c.parameterTypes(1) == classOf[String] then "abc" else 5
    println(c.newInstance(outer, arg).asInstanceOf[Plugin].name)
