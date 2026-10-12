package inl

// Inferred definitions whose pickled types the downstream reads exactly (invariant containers):
// a constant typed by its constant and widened where inferred, a folded constant kept by a
// `final val`, an element left open by `List.empty` and maximised by the lambda, one `List()`
// leaves to the improvement, a contravariant variable instantiated from above.
class Inv[T](val x: T)
class Sink[-T]
object Defs:
  final val sum = 1 + 2
  final val f = 1.5f * 2
  val widened = 1 + 2
  def inv[T](t: T): Inv[T] = new Inv(t)
  def fromEmpty = Inv(List(1).foldLeft(List.empty)((acc, x) => x :: acc))
  def fromApply = Inv(List(1).foldLeft(List())((acc, x) => x :: acc))
  def fromSet = List(1).foldLeft(Set())((acc, x) => acc + x)
  def literal = inv(1)
  def sink[T](t: T): Sink[T] = new Sink[T]
  def contra = sink(1)
  def pair = (1, "a")
