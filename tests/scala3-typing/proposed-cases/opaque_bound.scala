object Ids:
  opaque type Pos <: Int = Int
  object Pos:
    def apply(i: Int): Pos = i

object Use:
  import Ids.*
  val q: Int = Pos(3) + 1

@main def run(): Unit = println(Use.q)
