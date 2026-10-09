// expect: 6:50: error: Structural access not allowed on method f because it has a parameter type with an unstable erasure
// The second code pass's `structural_abstract` program: an abstract type member has no stable erasure.
import scala.reflect.Selectable.reflectiveSelectable
trait Types:
  type A <: CharSequence
  def call(x: { def f(a: A): Int }, a: A): Int = x.f(a)
@main def run(): Unit = println("must reject")
