// A polymorphic function type's ClassTag is its FunctionN's only where the type has a stable erasure
// (`synthesizedClassTag`'s `hasStableErasure`, which reads a refinement through its parent): over a type parameter
// bounded by `PolyFunction` there is none, as scalac 3.8.4 reports (E172).
// expect: classtag_poly_function_param_parent.scala:8:
// expect: No ClassTag available for
import scala.reflect.ClassTag
object P:
  def g[F <: PolyFunction]: ClassTag[F { def apply(x: Int): Int }] = summon[ClassTag[F { def apply(x: Int): Int }]]
