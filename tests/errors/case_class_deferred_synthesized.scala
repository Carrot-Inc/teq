// A member the rule synthesizes beside an abstract case class's own deferred declaration of it is scalac's E120:
// `SyntheticMembers.existingDef` takes no deferred definition, so the synthesized member is entered beside the
// declaration and erasure finds the two alike. A deferred declaration a trait makes is implemented by the
// synthesized member (K). Master accepted every class here.
// expect: 14:16: error: Conflicting definitions:
// expect: override def toString(): String in class C and
// expect: override def toString: String in class C at line 14
// expect: 16:16: error: Conflicting definitions:
// expect: override def equals(x$0: Any): Boolean in class D and
// expect: 18:16: error: Conflicting definitions:
// expect: 20:16: error: Conflicting definitions:
// expect: override def canEqual(that: Any): Boolean in class F and
abstract case class C(n: Int):
  override def toString: String
abstract case class D(n: Int):
  override def equals(o: Any): Boolean
abstract case class E(n: Int):
  override def productArity: Int
abstract case class F(n: Int):
  override def canEqual(o: Any): Boolean
trait T:
  override def toString: String
case class K(n: Int) extends T
