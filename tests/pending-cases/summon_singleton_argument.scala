// `summon[T]` is `x.type` of the given it finds (`transparent inline def summon[T](using x: T): x.type`), so it is
// passed where that given's singleton is wanted; teq types it `T` (a mismatch). Met by the probes of the review of
// `16412b2e9`.
object Test {
  given ord: Ordering[String] = Ordering.String
  def same(o: ord.type): Int = 1
  def main(args: Array[String]): Unit = println(same(summon[Ordering[String]]))
}
