/*
rules = DisableSyntax
DisableSyntax.noVars = true
DisableSyntax.noFinalize = true
DisableSyntax.noWhileLoops = true
DisableSyntax.noReturns = true
DisableSyntax.regex = [{ id = mutable, pattern = "(?m)^import scala\\.collection\\.mutable", message = "no mutable imports" }]
*/
// DisableSyntax's checks asked their settings first (fix-speed's table, §8): every check over one input, here under
// the application's settings; its lints recorded by the rule before the change.
package guards

import scala.collection.mutable // assert: DisableSyntax.mutable

object Guards:
  var v = 1 // assert: DisableSyntax.var
  val n: String = null
  def t(): Nothing = throw new Exception
  def r(): Int = return 1 // assert: DisableSyntax.return
  def w(): Unit = while v < 0 do v += 1 // assert: DisableSyntax.while
  def c(a: Any) = a.asInstanceOf[Int]
  def m(a: Any) = a.asInstanceOf[Matchable]
  def i(a: Any) = a.isInstanceOf[String]
  val s = 1; val s2 = 2
  val tab = 1	
  class Cov[+A]
  class Contra[-A]
  def d(a: Int = 1) = a
  trait Tr { val tv = 1 }
  implicit object IO
  implicit def conv(x: Int): String = ""
  final val fv = 1
  val (pa, pb) = (1, 2)
  val Some(pc) = Option(1)
  def eq(a: Int) = a == 1
  def ne(a: Int) = a != 1
  def finalize(): Unit = () // assert: DisableSyntax.noFinalize
  lazy val lz = 1
