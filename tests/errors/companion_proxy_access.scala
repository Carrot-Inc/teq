// expect: 11:16: error: object Val in class Enumeration does not take parameters
// expect: 1 error found

// The constructor proxy a companion without an `apply` gets has the class's access
// (`NamerOps.addConstructorApplies`): a protected class's is a protected member of the companion,
// out of a subclass's reach (E050).
class Enumeration:
  protected class Val(i: Int)
  object Val
class Sub extends Enumeration:
  val hearts = Val(27)
