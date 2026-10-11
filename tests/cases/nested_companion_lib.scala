// jars: nestedcompanion-lib
// Case classes nested in traits of a scalac-compiled jar, their companions applied in the traits'
// own bodies and inside the companions themselves (chimney's `TransformerFlags`, `ForTotal.create`
// and `TransformerOverride.Fallback`): the companion is a member of the enclosing class, found
// through its members, and its `apply` a member of the companion, constructing inside it, with
// defaults and a second clause; run by the jar's macro at compile time and called at run time.
import nclib.*

@main def run(): Unit =
  println(Macros.derive(3))
  println(Derivation.run(2))
  println(Derivation.Flags.global.deeper)
  println(Derivation.fallback(List(7)))
