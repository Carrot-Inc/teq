// expect: 11:7: error: covariant type A occurs in contravariant position in type => AnyRef{def take(x: A): Unit} of method take
// expect: 14:7: error: contravariant type A occurs in covariant position in type => AnyRef{def get: A} of method get
// expect: 17:7: error: covariant type A occurs in invariant position in type => AnyRef{type T = A} of method alias
// expect: 20:7: error: covariant type A occurs in contravariant position in type => AnyRef{type T >: A} of method lower
// absent: 22:
// A refinement in a member's type is folded as dotty's `VarianceChecker.Validator` folds a
// `RefinedType` (`TypeAccumulator.foldOver`): a method's parameters flipped, its result as it
// stands, an alias invariantly, a lower bound flipped and an upper bound as it stands.
class Cov[+A]:
  // a covariant parameter in a refinement method's parameter
  def take: AnyRef { def take(x: A): Unit } = ???
class Contra[-A]:
  // a contravariant parameter in a refinement method's result
  def get: AnyRef { def get: A } = ???
class Alias[+A]:
  // an alias is invariant
  def alias: AnyRef { type T = A } = ???
class Lower[+A]:
  // a lower bound is contravariant
  def lower: AnyRef { type T >: A } = ???
class Fine[+A]:
  def fine: AnyRef { def get: A; type U <: A; def take(f: A => Unit): Unit } = ???
