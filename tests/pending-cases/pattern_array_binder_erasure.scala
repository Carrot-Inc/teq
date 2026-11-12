// A binder of a constructor pattern over a scrutinee of
// type `Any` takes the class's type argument as a fresh
// type (dotty's `Wrapped[A$1]` of `typedUnApply`), so
// `arr: Array[A$1]` erases to `Object` and reads an `int[]`.
// tyrian-next's `Batch.equals` has this shape over a `js.Array`.
final case class Wrapped[A](values: Array[A]):
  override def equals(that: Any): Boolean = that match
    case Wrapped(arr) => values.sameElements(arr)
    case _ => false

@main def run(): Unit =
  println(Wrapped(Array(1, 2)) == Wrapped(Array(1, 2)))
  println(Wrapped(Array(1, 2)) == Wrapped(Array("1", "2")))
