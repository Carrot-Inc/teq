package tsa

// A parameter of a `*:` chain ending in the `EmptyTuple` alias, which `TypeErasure.tupleArity` reads as written
// (TypeErasure.scala:82-95): no tuple of a known arity, so the parameter erases to Product and `x._1` selects on the
// chain cast to `(Int, Int)` (`Typer.typedSelectWithAdapt`'s small generic tuple), a cast kept. Read back from the
// pickle, the chain keeps its alias, as the source's spelled chain does, and the method's signature is
// `head(scala.Product)`, `sigName`'s `erasePair` of the chain as written.
object Lib:
  def head(x: Int *: Int *: EmptyTuple): Int = x._1
  def useHead(): Int = head((3, 4))
