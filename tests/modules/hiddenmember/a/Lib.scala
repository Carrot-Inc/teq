package hiddenmember

// A member an abstract redeclaration hides is synthesized (dotty's `existingDef` finds a deferred
// member), so the rule's `toString` and `productPrefix` answer, not the hidden ancestor's bodies,
// through a trait's chain and a class's.
trait A:
  override def toString: String = "inherited"
  def productPrefix: String = "A-prefix"
trait B extends A:
  override def toString: String
  override def productPrefix: String
case class C(x: Int) extends B

abstract class K:
  override def toString: String = "class-inherited"
abstract class L extends K:
  override def toString: String
case class D(x: Int) extends L
