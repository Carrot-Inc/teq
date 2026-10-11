package roa

// An abstract redeclaration of an overloaded name's alternative overrides that alternative's body alone (dotty's
// `MixinOps.isInImplementingClass` over the alternatives of `nonPrivateMember`): a downstream class mixing `O2` prints
// as `Object` does, its `toString(n: Int)` stays `O1`'s.
trait O1:
  override def toString: String = "wrong"
  def toString(n: Int): String = "number"
trait O2 extends O1 { override def toString: String }
