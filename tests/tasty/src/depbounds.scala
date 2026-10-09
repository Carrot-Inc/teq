// The bounds of a method type's type parameters under substitution, and a polymorphic function's
// parameters as named (`TypeMap.mapOverLambda`, `Desugar.makePolyFunctionType`): the module cases
// of tests/modules (bounds, boundsinferred, boundspoly, nestedbounds, syntheticnamed,
// syntheticunnamed) hold the same shapes with their downstreams.
package fix.depbounds

trait DbCtx:
  type T

trait DbParent[T]:
  def run[A <: T](a: A): Any

// A generic member's bound naming the method's parameter, declared, inferred through a call and
// inside an enclosing refinement method's result; a polymorphic function type's bound alike.
object DbBounds:
  def keep(c: DbCtx)(f: DbParent[c.T] { def run[A <: c.T](a: A): A }) = f
  def make(c: DbCtx): DbParent[c.T] { def run[A <: c.T](a: A): A } = ???
  def inferred(c: DbCtx) = make(c)
  def nested(f: AnyRef { def outer(c: DbCtx)(x: c.T): DbParent[c.T] { def run[A <: c.T](a: A): A } }) = f
  def poly(c: DbCtx): [A <: c.T] => (a: A) => a.type = ???
  def polyInferred(c: DbCtx) = poly(c)

// A polymorphic function's parameters as the source names them: `x$1` written, none (scalac's
// `x$1`), a literal's own.
object DbPolyNames:
  def written(f: [A] => (x$1: A) => A) = f
  def unnamed(f: [A] => A => A) = f
  val literal = [A] => (a: A, b: A) => a
  val contextual = [A] => (a: A) ?=> a
