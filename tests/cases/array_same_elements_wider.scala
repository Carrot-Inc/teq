// scala-library's `ArrayOps.sameElements[B >: A](that: IterableOnce[B])`: an array compared with
// elements of a wider type, an array of them converted, as tyrian-next's `Batch.equals` compares
// its wrapped array with one a pattern binds (tests/pending-cases/pattern_array_binder_erasure.scala).
@main def run(): Unit =
  val a: Array[Int] = Array(1, 2)
  val b: Array[Any] = Array(1, 2)
  println(a.sameElements(b))
  println(a.sameElements(List[AnyVal](1, 2)))
  println(a.sameElements(Vector(1)))
