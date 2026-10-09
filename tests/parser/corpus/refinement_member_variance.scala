// A variable used contravariantly and covariantly, the covariant use a refinement's member,
// is not used only contravariantly: it is minimised to `Nothing`, as scalac interpolates it,
// not maximised to `Any` (`Out { val value: A }` beside an `In[A]`, also through an alias).
trait In[-A]
class Out(val tag: String)
def inner[A]: In[A] = new In[A] {}
def mkOut[A]: Out { val value: List[A] } = new Out("out") { val value: List[A] = Nil }
def outer[A](x: In[A]): (In[A], Out { val value: List[A] }) = (x, mkOut[A])
def mkFirst[A]: Out { def first: Option[A] } = new Out("aliased") { def first: Option[A] = None }
type Refined[A] = Out { def first: Option[A] }
def aliased[A](x: In[A]): (In[A], Refined[A]) = (x, mkFirst[A])

@main def run(): Unit =
  val result = outer(inner)
  val check: Out { val value: List[Nothing] } = result._2
  println(check.tag)
  val other = aliased(inner)
  val checkAliased: Out { def first: Option[Nothing] } = other._2
  println(checkAliased.tag)
