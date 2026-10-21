/*
rules = DisableSyntax
DisableSyntax.noNulls = true
DisableSyntax.noTabs = true
DisableSyntax.noCovariantTypes = true
DisableSyntax.noUniversalEquality = true
DisableSyntax.noImplicitObject = true
DisableSyntax.noValPatterns = true
*/
// DisableSyntax's checks asked their settings first (fix-speed's table, §8): every check over one input, here under
// a mixed set of settings; its lints recorded by the rule before the change.
package guards

import scala.collection.mutable

object Guards:
  var v = 1
  val n: String = null // assert: DisableSyntax.null
  def t(): Nothing = throw new Exception
  def r(): Int = return 1
  def w(): Unit = while v < 0 do v += 1
  def c(a: Any) = a.asInstanceOf[Int]
  def m(a: Any) = a.asInstanceOf[Matchable]
  def i(a: Any) = a.isInstanceOf[String]
  val s = 1; val s2 = 2
  val tab = 1	// assert: DisableSyntax.noTabs
  class Cov[+A] // assert: DisableSyntax.covariant
  class Contra[-A]
  def d(a: Int = 1) = a
  trait Tr { val tv = 1 }
  implicit object IO // assert: DisableSyntax.implicitObject
  implicit def conv(x: Int): String = ""
  final val fv = 1
  val (pa, pb) = (1, 2)
  val Some(pc) = Option(1) // assert: DisableSyntax.noValPatterns
  def eq(a: Int) = a == 1 // assert: DisableSyntax.==
  def ne(a: Int) = a != 1 // assert: DisableSyntax.!=
  def finalize(): Unit = () // assert: DisableSyntax.noFinalize
  lazy val lz = 1
