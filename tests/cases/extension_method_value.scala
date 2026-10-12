// An object's extension method named with no argument is the method its receiver's clause is the
// first of, eta-expanded (dotty's etaExpand): lila's fullId.map(GameFullId.playerId).
object Ops:
  extension (e: String)
    def twice: String = e + e
    def times(n: Int): String = e * n
  extension [A](xs: List[A])
    def second: A = xs(1)

object ids:
  opaque type FullId = String
  object FullId:
    def apply(s: String): FullId = s
    extension (e: FullId)
      def playerId: String = e.drop(2)

@main def run(): Unit =
  println(Ops.twice("ab"))
  println(List("a", "b").map(Ops.twice))
  val f: String => String = Ops.twice
  println(f("q"))
  val g = Ops.twice
  println(g("r"))
  val t: String => Int => String = Ops.times
  println(t("x")(3))
  println(List(List(1, 2), List(3, 4)).map(Ops.second))
  println(Option(ids.FullId("abcd")).map(ids.FullId.playerId))
