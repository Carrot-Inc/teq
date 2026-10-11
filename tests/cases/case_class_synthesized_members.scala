// The members a case class takes by rule (dotty's `SyntheticMembers.caseAndValueMethods`) are
// those it does not define (`existingDef`): the rule's `toString` reads the class's own
// `productPrefix` and elements (`ScalaRunTime._toString`), its `equals` ends with
// `that.canEqual(this)`, and a call through `Product` dispatches to the class's members.
case class N(x: Int):
  override def canEqual(that: Any): Boolean = false
  override def productPrefix: String = "Nn"
case class P(x: Int) extends Product:
  override def productArity: Int = 2
  override def productElement(n: Int): Any = n + 8
case class Q(x: Int):
  override def productIterator: Iterator[Any] = Iterator(99)
case class R(x: Int)(val tag: Int):
  override def canEqual(that: Any): Boolean = { print(tag); true }
case class S(x: Int):
  override def toString: String = "own"
  override def equals(x: Any): Boolean = false
  override def hashCode: Int = 42
case class K(x: Int)
class D(x: Int) extends K(x):
  override def productPrefix: String = "Dd"

@main def run(): Unit =
  val n = N(1)
  println(n)
  println(n == N(1))
  println(n == n)
  println((n: Product).canEqual(N(1)))
  println(P(1))
  println((P(1): Product).productArity)
  println((P(1): Product).productElement(0))
  println(Q(1))
  println((Q(1): Product).productIterator.next())
  println(R(1)(2) == R(1)(3))
  println(S(1))
  println(S(1) == S(1))
  println(S(1).hashCode)
  println(new D(3))
  println(new D(3) == K(3))
  println(K(3) == new D(3))
