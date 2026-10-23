import scala.annotation.StaticAnnotation
class Mark extends StaticAnnotation
class Box[A]
class Out[+A]
class In[-A]
class C { type T }
object Aliases {
  type A[T] = Box[T] @Mark
  val annotated: A[Int] = ???
  def map[X, Y](a: X)(k: X => Y): Y = k(a)
  type Inv[T] = Box[T]
  val inInv = map(new C)(x => (??? : In[Inv[x.T]]))
}
