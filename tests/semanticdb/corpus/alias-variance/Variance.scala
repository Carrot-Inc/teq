class Out[+A]
class In[-A]
class Box[A]
class C { type T }
object Variance {
  type Co[+A] = Out[A]
  type Contra[-A] = In[A]
  type Inv[A] = Box[A]
  def map[X, Y](a: X)(k: X => Y): Y = k(a)
  val co = map(new C)(x => (??? : Co[x.T]))
  val contra = map(new C)(x => (??? : Contra[x.T]))
  val inv = map(new C)(x => (??? : Inv[x.T]))
  val nested = map(new C)(x => (??? : Box[Contra[x.T]]))
}
