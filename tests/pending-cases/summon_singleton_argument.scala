// `summon[T]` is `x.type` of the given it finds
// (`transparent inline def summon[T](using x: T): x.type`), so it is
// passed where that given's singleton is wanted; teq is to type it so too.
object Test {
  given ord: Ordering[String] = Ordering.String
  def same(o: ord.type): Int = 1
  def main(args: Array[String]): Unit = println(same(summon[Ordering[String]]))
}
