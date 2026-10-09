// A polymorphic function's type parameters with both bounds, as its type, a literal and a
// generic refinement member write them (dotty's `Parsers.typeBounds`), and under substitution:
// the module cases of tests/modules (polylower, boundslower) hold the same shapes with their
// downstreams (polyval a `PolyFunction` refinement beside a `val`, which the lean std this unit is
// checked over has no class for).
package fix.polybounds

trait PbCtx:
  type T

trait PbParent[T]:
  def run[A >: T](a: A): Any

object PbLower:
  def keep(f: [A >: String] => (a: A) => A) = f
  def both(f: [A >: Null <: AnyRef] => (a: A) => A) = f
  def member(c: PbCtx)(f: PbParent[c.T] { def run[A >: c.T](a: A): A }) = f
  def poly(c: PbCtx): [A >: c.T] => (a: A) => a.type = ???
  def inferred(c: PbCtx) = poly(c)
  val literal = [A <: Int] => (a: A) => a + 1
  val lowLiteral = [A >: String] => (a: A) => a
