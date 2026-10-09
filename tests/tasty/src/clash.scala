// Compiled with Scala 3.8.4: an anonymous class of a body that overrides a method one parent
// overloads and the other does not, run by a macro at compile time
// (`tests/classpath/jvm/jvm_link_body_name_clash.scala`).
package fix.clash

import scala.quoted.*

trait ClashSingle:
  def pick(x: Int): Int
trait ClashOverloaded:
  def pick(x: Int): Int
  def pick(x: String): String

object ClashMaker:
  def make: ClashSingle & ClashOverloaded = new ClashSingle with ClashOverloaded:
    def pick(x: Int): Int = x + 1
    def pick(x: String): String = x + "!"

  inline def picked(inline n: Int): String = ${ pickedImpl('n) }
  def pickedImpl(n: Expr[Int])(using Quotes): Expr[String] =
    val m = make
    Expr(s"${m.pick(1)} ${m.pick("a")}")
