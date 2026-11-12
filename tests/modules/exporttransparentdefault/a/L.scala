// An inline parameter's default getter is an inline method of its own (Desugar.scala:429), not transparent: in a
// downstream's typing of a transparent expansion its call stays a call, for the inlining phase
// (`Inlines.needsInlining`), and the downstream's pickle selects it by its name, a default getter's name of its
// method's (`NameBuffer.nameIndex`'s DerivedName arm), on the receiver: `Fw.tr()`, through the export's forwarder,
// and `tt`, a transparent inline def calling the method on the object's val. A reader of that pickle is to find
// the member by the name (scalac 3.8.4's -from-tasty, the next module's typing of the body read from the products).
package etd

class Box(val k: Int):
  transparent inline def tr(inline x: Int = k + 3): Int = x * 10
  inline def p(inline x: Int = k + 5): Int = x * 100

object Fw:
  val bx = Box(4)
  export bx.tr

transparent inline def tt: Int = Fw.bx.p() + 1
