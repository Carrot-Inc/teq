// A downstream of a/ as tests/modules builds it. scalac 3.8.4 prints the .expected, and so does each of teq's runs.
// a/'s `boxed`, a cast to a type parameter bounded by BoxedUnit, returns nothing in teq's whole Scala.js bundle
// (`return;`) and `undefined` in the bundle built over a/'s lean products (`return undefined;`): the bundles differ
// (tests/modules' bundle: one file, --split and --release alike), the program's output the same. The same on
// integration 5745c6e758. Split off products_tuple_selection_cast, whose tuple cast tests/modules/tupleselectioncast
// holds since the workers exports helper's push 11.
// What the emitter is given (traced at `emit_body`): the whole build's body of `boxed` is `Literal(Unit)` of type `A`,
// the source typer's cast folded and `x` dropped; the products build's, re-typed from a/'s pickle, `Cast(x, Erased,
// A)`, which the emitter writes `undefined`. Master efddb38de3 writes `x; return;` in both bundles, which agree: a
// regression. The mechanism: a/'s pickle holds `().asInstanceOf[A]` (`UNITconst` as the operand), where scalac pickles
// the typer's `x.asInstanceOf[A]`. teq lowers the cast while typing (`cast_lowering`'s `CastLowering::Unit`, the target
// erasing to Unit): `boxing_constant` drops the pure operand `x` (Erasure.Boxing.constant), and the capture's
// `Form::Cast(t)` lands on the remaining `()` (`lower_cast_as`, prims.rs), so the writer writes that literal as the
// cast's operand; the products build re-types `().asInstanceOf[A]` (from Unit, `Same`) and keeps an erased cast. A
// capture of the cast needs the operand the typer dropped, as dotty's pickle precedes Erasure. Routed (prims.rs).
package bxb

import bxa.Lib

@main def run(): Unit =
  val u = Lib.boxed[scala.runtime.BoxedUnit](null)
  println("boxed")
