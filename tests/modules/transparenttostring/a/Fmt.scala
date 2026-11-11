// A transparent body's `toString` on a type parameter, `Any`'s member as dotty's lookup finds it on `Int`, `Boolean`
// or `String` once the expansion instantiates the parameter (`TermRef.withPrefix`): the expansion keeps the
// universal conversion, as the typer does where its lookup finds `Any`'s symbol, and the downstream's products
// pickle it as scalac's `SELECTin toString` of `scala.Any` (TreePickler.pickleTree's Select case). Its classes,
// subset and check-lean-whole rows fail on two older departures, the whole build's INLINED layers
// (tests/pending-cases/products_transparent_expansion_inlined_layers) and the inlined argument markers the build
// over the products drops (products_inlined_argument_marker): with the INLINED nodes set aside, b/'s whole and split
// pickles are identical.
package tcoa

object Fmt:
  transparent inline def show[T](inline label: String, x: Int, y: T): String =
    val pre = label + ": "
    val k = pre.length + x
    pre + x.toString + (if k > 10 then "!" else ".") + k.toString + " " + y.toString
  transparent inline def plain[T](y: T): String = y.toString
