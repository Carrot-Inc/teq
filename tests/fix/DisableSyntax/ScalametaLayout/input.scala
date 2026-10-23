/*
rules = DisableSyntax
DisableSyntax.noDefaultArgs = true
DisableSyntax.noImplicitConversion = true
DisableSyntax.noValInAbstract = true
*/
// DisableSyntax's checks over scalameta 4.17's layout, as its source reads it (DisableSyntax.scala at 41cf7983): a
// secondary constructor is a `Ctor.Secondary`, no `Defn.Def`, so its default is no `DefaultArgs`'s; a default in braces
// is its `Term.Block`; a parameter of an `implicit` clause carries `Mod.Implicit` and one of a `using` clause `Mod.Using`,
// so `hasNonImplicitParam` holds of the second alone; a template's statements are its `Template.Body`'s, past its self.
package layout

class K(a: Int):
  def this(a: Int, b: Int = 2) = this(a)
  def braced(x: Int = { 1 }) = x /* assert: DisableSyntax.defaultArgs
                      ^^^^^
*/

object Conversions:
  implicit def allImplicit(implicit foo: Int): String = foo.toString
  implicit def usingOne(using foo: Int): String = foo.toString // assert: DisableSyntax.implicitConversion

abstract class A { self =>
  val x = 1 // assert: DisableSyntax.valInAbstract
}
