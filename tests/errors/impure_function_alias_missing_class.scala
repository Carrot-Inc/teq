// An impure alias whose underlying class does not exist (the pass's BadImpure.scala on fddacdde0): `ImpureFunction01`
// is a synthetic name, its `Function01` no function class. `newFunctionNType`'s completer reaches `requiredClass` →
// `requiredSymbol`, whose `TypeError` (`package scala does not have a member class Function01`) its caller drops; the
// alias's completion, started and never ended, answers the reference's read with `CyclicReference`: E046 at the
// reference, as scalac 3.8.4 gives it.
// expect: 7:27: error: Cyclic reference involving type ImpureFunction01
object BadImpure { val f: scala.ImpureFunction01[Int, Int] = null }
