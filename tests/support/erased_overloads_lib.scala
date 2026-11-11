// The jar of tests/cases/library_overloads_erased_alike.scala: overloads scala-library's erasure tells apart
// (`Seq[Any]` and `Array[Any]`), which a class compiled from the jar's TASTy keeps whatever teq's std erases them to.
package erasedoverloads

abstract class Maker[T] {
  def rawMake(values: Seq[Any]): T
  def rawMake(values: Array[Any]): T = rawMake(values.toSeq)
}

final class PairMaker extends Maker[(Any, Any)] {
  def rawMake(values: Seq[Any]): (Any, Any) = (values(0), values(1))
}
