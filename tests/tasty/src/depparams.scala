// Method types inside types: a refinement's method whose parameters a later parameter or the
// result names (`PARAMtype` to its `METHODtype`), its by-name and repeated parameters, a generic
// refinement member (`POLYtype` over the `METHODtype`), a polymorphic function type's `apply`
// with its names and dependent result, and a dependent context function type's parent. The
// module cases of tests/modules (depfn, deptypes, polydep, ...) hold the same shapes with their
// downstreams.
package fix.depparams

trait DpCtx:
  type T
  val t: T

// The TASTy inspector's shape over the library's own types.
object DpFn:
  def inspect[T](files: List[String])(fn: (ctx: String) ?=> List[Option[ctx.type]] => T): T =
    given s: String = files.mkString(",")
    fn(Nil)

// An alias, an annotated val and an inferred one, an override and an inherited member.
object DpTypes:
  type F = (c: DpCtx) => c.T
  val explicit: (c: DpCtx) => c.T = c => c.t
  val inferred = explicit

trait DpBase:
  def value: (c: DpCtx) => c.T

class DpDerived extends DpBase:
  def value = DpTypes.inferred
  val inherited = superValue
  def superValue = value

abstract class DpInherited extends DpBase:
  val inferred = value

// A refinement's method whose parameter types or result name an earlier parameter, in its own
// clause and in a later one; a dependent function type whose result is another.
object DpClauses:
  def paramOnly(f: AnyRef { def run(c: DpCtx)(x: c.T): Int }): Unit = ()
  def sameParamOnly(f: AnyRef { def run(c: DpCtx, x: c.T): Int }): Unit = ()
  def cross(f: AnyRef { def run(c: DpCtx)(x: c.T): c.T }) = f
  def sameClause(f: AnyRef { def run(c: DpCtx, x: c.T): c.T }) = f
  def nested(f: (c: DpCtx) => (x: c.T) => c.T) = f

// A refinement's by-name parameter, dependent or not.
object DpModes:
  def byName(f: AnyRef { def run(x: => Int): Int }) = f
  def byNameDep(f: AnyRef { def run(c: DpCtx)(x: => c.T): c.T }) = f

trait DpParent:
  def run[A](a: A): Any
  def unused[A](a: Int): Any
  def gen[A]: Any
  def rep(xs: Int*): Int

// Generic refinement members, and a repeated parameter (`<repeated>[Int]` in the method type).
object DpMembers:
  def member(f: DpParent { def run[A](a: A): A }) = f
  def memberDep(f: DpParent { def run[A](a: A): a.type }) = f
  def unused(f: DpParent { def unused[A](a: Int): Int }) = f
  def gen(f: DpParent { def gen[A]: A }) = f
  def rep(f: DpParent { def rep(xs: Int*): Int }) = f

// Polymorphic function types: a dependent result, unnamed parameters, named ones.
object DpPoly:
  def dep(f: [A] => (a: A) => a.type) = f
  val f: [A] => (a: A) => a.type = [A] => (a: A) => a
  def plain(f: [A] => (A, A) => A) = f
  def named(f: [A] => (first: A, second: A) => A) = f

// The parent of a dependent context function type, `Nothing` under `Option` in the inner
// function's parameter.
object DpApprox:
  def widen[T](f: (s: String) ?=> List[Option[s.type]] => T): String ?=> List[Option[Nothing]] => T = f
