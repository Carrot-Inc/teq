// Compiled with Scala 3.8.4: inline methods declared without a result type, which a pickle
// writes with the result scalac infers at the definition, the one every call then sees:
// the body's type widened, an override's the type of the overridden member.
package fix.inlres

trait InlT:
  def f: Any
  def plain(x: Int): String

object InlRes:
  inline def make(x: Int) = { def local() = x; () => local() }
  transparent inline def pick(inline b: Boolean) = inline if b then 1 else "one"
  inline def pickPlain(inline b: Boolean) = inline if b then 1 else "one"
  inline def lit = 5
  inline def str(x: Int) = s"v $x"

class InlD extends InlT:
  inline def f = 1
  inline def wrap(x: Int): String = s"<$x>"
  inline override def plain(x: Int) = wrap(x)
