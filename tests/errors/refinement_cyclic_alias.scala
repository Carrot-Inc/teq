// expect: 13:22: error: illegal cyclic type reference: alias A of type A refers back to the type itself
// expect: 14:26: error: illegal cyclic type reference: alias X of type X refers back to the type itself
// expect: 15:26: error: illegal cyclic type reference: alias L of type L refers back to the type itself
// expect: 18:26: error: illegal cyclic type reference: alias A of type A refers back to the type itself
// expect: 13:26: warning: forward reference in refinement is deprecated
// expect: 16:35: warning: forward reference in refinement is deprecated
// absent: 16:26: error
// An alias of a refinement that comes back to itself through the refinement's aliases is
// reported once, at the alias whose completion meets the cycle (dotty's completion of the
// refinement class's alias, `Checking.checkNonCyclic`, E140), through a nested recursive
// refinement too, which exempts nothing; an alias over the others' is none.
object Foo:
  def test(f: { type A = A }): Int = ???
  type B = AnyRef { type X = Y; type Y = X }
  type C = AnyRef { type L = List[L] }
  type D = AnyRef { type M = List[N]; type N = Int }
  // through a nested recursive refinement:
  type N = AnyRef { type A = AnyRef { type D; type C = (A, D) } }
  // through a match alias's scrutinee, directly and through another alias (`MatchAlias`,
  // `atVariance(0)(checkUpper(..))`); a case's body may name the alias:
  type MA = AnyRef { type A = A match { case String => Int } }
  type MB = AnyRef { type A = B match { case String => Int }; type B = A }
  type MC = AnyRef { type A[X] = X match { case String => Int; case List[t] => A[t] } }
// expect: 21:27: error: illegal cyclic type reference: alias A of type A refers back to the type itself
// expect: 22:27: error: illegal cyclic type reference: alias A of type A refers back to the type itself
// absent: 23:27: error
