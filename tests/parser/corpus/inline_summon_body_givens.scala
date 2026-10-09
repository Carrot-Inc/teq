// `summonInline` searches where the body expands, in the scopes the expanded bodies open around it:
// a given the body defines stands nearer than the call site's, one the definition's enclosing
// object defines is not in scope, and an inline argument's search is its call site's, as scalac
// 3.8.4 expands them (`{ given Int = 7; summonInline[Int] }` is 7 wherever it expands). A scope
// ends with its block: the call after `{ given Int = 7; get }` finds the caller's given.
import scala.compiletime.summonInline

object Defs:
  given Int = 5
  inline def local: Int = { given Int = 7; summonInline[Int] }
  inline def fromArg(inline x: Int): Int = { given Int = x; summonInline[Int] }
  inline def mixed: String = { given String = "local"; summonInline[String] + local }
  inline def inner: Int = summonInline[Int]
  inline def outer: Int = { given Int = 8; inner }
  inline def nested: Int = { given Int = 8; { given Int = 6; summonInline[Int] } }
  inline def viaArg(inline x: Int): Int = { given Int = 4; x }
  inline def after: Int = {
    { given Int = 7; inner }
    inner
  }

object Use:
  given Int = 9
  def site = Defs.inner
  def local = Defs.local
  def outer = Defs.outer
  def nested = Defs.nested
  def viaArg = Defs.viaArg(summonInline[Int])
  def after = Defs.after

@main def run(): Unit =
  println(Use.site)
  println(Use.local)
  println(Defs.fromArg(3))
  println(Defs.mixed)
  println(Use.outer)
  println(Use.nested)
  println(Use.viaArg)
  println(Use.after)
