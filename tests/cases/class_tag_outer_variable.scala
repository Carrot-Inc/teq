// `xs.toArray` given for a generic `Array[A]` parameter (indigo's `Batch.fromArray(range.flatMap(..).toArray)`): the tag's
// variable, bounded below by the element type and tied to the enclosing call's open `A`, is made fully defined from
// that type, as dotty's `Synthesizer.synthesizedClassTag` instantiates it (`instArg`, `isFullyDefined(tp, ForceDegree.all)`).
final case class Batch[A](items: List[A])
object Batch:
  def fromArray[A](values: Array[A]): Batch[A] = Batch(values.toList)

@main def run(): Unit =
  buttons()
  println(Batch.fromArray(Vector("a").toArray))
  println(Batch.fromArray(List(1, 2).map(_.toString).toArray))
  println(Batch.fromArray((0 to 2).map(_.toString).toArray))
  val xs: IndexedSeq[String] = Vector("b")
  println(Batch.fromArray(xs.toArray))

enum Button:
  case Left, Right, Middle
object Button:
  def fromOrdinalOpt(i: Int): Option[Button] = if i >= 0 && i < 3 then Some(Button.fromOrdinal(i)) else None

def buttons(): Unit =
  println(Batch.fromArray((0 to 5).filter(i => i % 2 == 0).flatMap(Button.fromOrdinalOpt).toArray))
