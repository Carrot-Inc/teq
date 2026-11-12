// `ImpureFunction1[Int, Int]` is `(Int => Int)` with a retaining annotation
// (`RetainingAnnotation(RetainsCapAnnot)`), which keeps the two parameter types
// apart before erasure: scalac refuses the pair at erasure, E120's "the same
// type after erasure", and teq is to refuse it so too. Moves with the annotation
// carrier (`AnnotatedType` with `Annotation`'s states). scalac's outcome below.
class ClashImpure {
  def a(f: Int => Int): Int = 1
  def a(f: scala.ImpureFunction1[Int, Int]): Int = 2
}
