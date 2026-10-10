// expect: 11:22: error: illegal cyclic type reference: alias A of type A refers back to the type itself
// expect: 12:26: error: illegal cyclic type reference: alias X of type X refers back to the type itself
// expect: 13:26: error: illegal cyclic type reference: alias L of type L refers back to the type itself
// expect: 11:26: warning: forward reference in refinement is deprecated
// expect: 14:35: warning: forward reference in refinement is deprecated
// absent: 14:26: error
// An alias of a refinement that comes back to itself through the refinement's aliases is
// reported once, at the alias whose completion meets the cycle (dotty's completion of the
// refinement class's alias, E140); an alias over the others' is none.
object Foo:
  def test(f: { type A = A }): Int = ???
  type B = AnyRef { type X = Y; type Y = X }
  type C = AnyRef { type L = List[L] }
  type D = AnyRef { type M = List[N]; type N = Int }
