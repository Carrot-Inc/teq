// expect: 9:7: error: Conflicting definitions:
// expect: override def toString(): String in class C and
// expect: def toString[A]: String in class C at line 9
// expect: have the same type (): String after erasure.
// expect: 1 error found
// A deferred declaration with the synthesized member's erased signature is refused, its type
// parameters erased (`ElimErasedValueType.checkNoClashes`' `signature` comparison, E120).
abstract case class C(n: Int):
  def toString[A]: String
