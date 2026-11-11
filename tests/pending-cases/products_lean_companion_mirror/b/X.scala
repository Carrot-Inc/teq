// A downstream of a/ as tests/modules builds it. An inline method of a/ makes a value through the mirror it summons
// (`m.fromProduct`), for a case class with a written companion (`C`) and one without (`D`). scalac 3.8.4 prints the
// .expected, and so do teq's JVM runs and its whole Scala.js and interpreter runs. Over teq's products of a/ built with
// the lean std (tests/modules' js-run, interp-run), the companion read from the pickles is no mirror: the Scala.js run
// throws "ClassCastException: class C cannot be cast to class Product", the interpreter's "class zma.C$ cannot be
// cast to class scala.deriving.Mirror$Product". The whole build's classes also differ from the split build's (classes,
// subset, bundle). Cause not located. The same on integration b602956824; visible in tests/modules' inferredbindings,
// typenamedbinding, upstreaminline and writtenunion once the SELECTtpt port lets their products build.
package zmb
import zma.*
@main def run(): Unit =
  println(Defs.make[C])
  println(Defs.make[D])
