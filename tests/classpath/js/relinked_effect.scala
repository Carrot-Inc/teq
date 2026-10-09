// jars: fixtures fixtures-shadow
// The library of tests/tasty/src/effects.scala was compiled against an `EffDefaults` whose
// effect is a JavaScript function; the class path links another, whose effect is an `Option`.
// The types scalac inferred in its bodies name the first, and are inferred again where they do
// not check.
import fix.shadow.*

@main def run(): Unit =
  println(EffLib.runTwice(21))
  println(EffLib.empty.size)
  println(EffLib.slot(3).size)
  println(EffLib.scoped(1))
  println(EffCells.cell(7))
  println(EffLib.mapped(4))
  println(EffLib.sized(6))
  println(EffLib.viaVal(5))
  println(EffLib.viaNestedVals(6))
  println(EffLib.viaRunner(8))
  println(EffStates.bump(EffState(Some(1))).slot)
