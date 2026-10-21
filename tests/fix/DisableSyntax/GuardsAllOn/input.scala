/*
rules = DisableSyntax
DisableSyntax.noVars = true
DisableSyntax.noNulls = true
DisableSyntax.noThrows = true
DisableSyntax.noReturns = true
DisableSyntax.noWhileLoops = true
DisableSyntax.noAsInstanceOf = true
DisableSyntax.noIsInstanceOf = true
DisableSyntax.noSemicolons = true
DisableSyntax.noTabs = true
DisableSyntax.noCovariantTypes = true
DisableSyntax.noContravariantTypes = true
DisableSyntax.noDefaultArgs = true
DisableSyntax.noValInAbstract = true
DisableSyntax.noImplicitObject = true
DisableSyntax.noImplicitConversion = true
DisableSyntax.noFinalVal = true
DisableSyntax.noValPatterns = true
DisableSyntax.noUniversalEquality = true
DisableSyntax.noFinalize = true
DisableSyntax.keywords = [lazy]
DisableSyntax.regex = [{ id = mutable, pattern = "(?m)^import scala\\.collection\\.mutable", message = "no mutable imports" }]
*/
// DisableSyntax's checks asked their settings first (fix-speed's table, §8): every check over one input, here under
// every setting on; its lints recorded by the rule before the change.
package guards

import scala.collection.mutable // assert: DisableSyntax.mutable

object Guards:
  var v = 1 // assert: DisableSyntax.var
  val n: String = null // assert: DisableSyntax.null
  def t(): Nothing = throw new Exception // assert: DisableSyntax.throw
  def r(): Int = return 1 // assert: DisableSyntax.return
  def w(): Unit = while v < 0 do v += 1 // assert: DisableSyntax.while
  def c(a: Any) = a.asInstanceOf[Int] // assert: DisableSyntax.asInstanceOf
  def m(a: Any) = a.asInstanceOf[Matchable] // assert: DisableSyntax.asInstanceOfMatchable
  def i(a: Any) = a.isInstanceOf[String] // assert: DisableSyntax.isInstanceOf
  val s = 1; val s2 = 2 // assert: DisableSyntax.noSemicolons
  val tab = 1	// assert: DisableSyntax.noTabs
  class Cov[+A] // assert: DisableSyntax.covariant
  class Contra[-A] // assert: DisableSyntax.contravariant
  def d(a: Int = 1) = a // assert: DisableSyntax.defaultArgs
  trait Tr { val tv = 1 } // assert: DisableSyntax.valInAbstract
  implicit object IO // assert: DisableSyntax.implicitObject
  implicit def conv(x: Int): String = "" // assert: DisableSyntax.implicitConversion
  final val fv = 1 // assert: DisableSyntax.noFinalVal
  val (pa, pb) = (1, 2)
  val Some(pc) = Option(1) // assert: DisableSyntax.noValPatterns
  def eq(a: Int) = a == 1 // assert: DisableSyntax.==
  def ne(a: Int) = a != 1 // assert: DisableSyntax.!=
  def finalize(): Unit = ()
  lazy val lz = 1 // assert: DisableSyntax.lazy
