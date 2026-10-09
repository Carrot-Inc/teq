// expect: 15:67: error: Structural access not allowed on method f because it has a parameter type with an unstable erasure
// expect: 16:54: error: Structural access not allowed on method g because it has a parameter type with an unstable erasure
// A structural call through scala-library's reflective `Selectable` passes the classes of the
// method's parameters, which the parameter types alone must fix (dotty's
// `Dynamic.handleStructural`, `TypeErasure.hasStableErasure`): a type parameter or an abstract
// type member has none. A path, a refinement and a union of classes have one.
import scala.reflect.Selectable.reflectiveSelectable

class C:
  def f(x: CharSequence): Int = x.length()
trait Box:
  type T
  val t: T

def call[T <: CharSequence](x: { def f(a: T): Int }, a: T): Int = x.f(a)
def member(b: Box)(x: { def g(a: b.T): Int }): Int = x.g(b.t)
def stable(s: String)(x: { def h(a: s.type, b: AnyRef { def k: Int }, c: Int | String): Int }): Int = x.h(s, null, 1)
