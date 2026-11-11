// `ImpureFunction1[Int, Int]` is `(Int => Int)` with a retaining annotation (`RetainingAnnotation(RetainsCapAnnot)`),
// which keeps the two parameter types apart before erasure: scalac refuses the pair at erasure, E120's "the same type
// after erasure", where teq's types carry no such annotation outside semantic typing, so its typer refuses them as one
// method defined twice ("already defined"; the adversary's B-4 on the M2e refold). Moves with the annotation carrier
// (seam 2, `AnnotatedType` with `Annotation`'s states; the coordinator's ruling of 28 Oct). scalac's outcome below.
class ClashImpure {
  def a(f: Int => Int): Int = 1
  def a(f: scala.ImpureFunction1[Int, Int]): Int = 2
}
