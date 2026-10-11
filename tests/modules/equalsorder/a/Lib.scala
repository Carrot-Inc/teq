package equalsorder

// A case class's `equals` by rule compares its primitive fields before the others
// (`SyntheticMembers.equalsBody`'s `sortedAccessors`), so an unequal `n` decides before `a`'s
// `equals` runs, here and through the products.
class X:
  override def equals(that: Any): Boolean = { print("effect;"); true }
case class C(a: X, n: Int)
case class D(a: X, b: String, n: Long, c: X, m: Boolean)
