// An upstream's transparent bodies, an interpolation among them, expanded in a middle module's
// products: the pickle writes the expansion's `StringContext.apply(parts*).s(args*)` with the parts and
// the arguments in their roles, as dotty's typed application and `TreePickler.pickleTree` keep them,
// whatever types the record's copy gives the parts.
package iia

object Lib:
  transparent inline def text(inline x: Int): String = s"number=$x"
  transparent inline def both(inline x: Int, inline y: String): String = s"$x and $y"
  transparent inline def list(inline x: Int): List[Int] = List(x, x + 1)
  transparent inline def choice(inline b: Boolean): Any = inline if b then 3 else "str"
