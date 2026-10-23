// expect: 19:16: error: type mismatch: found In[Nothing], required Int
// expect: 21:16: error: type mismatch: found Box[Nothing], required Int
// expect: 2 errors found
// A closure's result whose escaping parameter's member sits in an invariant argument under a
// contravariant one: the avoidance's range is `Nothing & Box[?]` at the negative variance
// (`ApproximatingTypeMap.range`), and the inferred type is simplified as dotty simplifies it
// (`TypeOps.simplify` at `Namer.inferredResultType`): `In[Nothing]`, as scalac infers it. An
// annotated intersection with `Nothing` simplifies to `Nothing` (`&`): `Box[Nothing]`, as scalac
// infers it.
import scala.annotation.StaticAnnotation
class In[-A]
class Box[A]
class C { type T }
class Mark extends StaticAnnotation
object InferredRange {
  type Inv[T] = Box[T]
  def map[X, Y](a: X)(k: X => Y): Y = k(a)
  val inInv = map(new C)(x => (??? : In[Inv[x.T]]))
  val u: Int = inInv
  val bottom = map(new C)(x => (??? : Box[(Nothing & String & x.T) @Mark]))
  val v: Int = bottom
}
