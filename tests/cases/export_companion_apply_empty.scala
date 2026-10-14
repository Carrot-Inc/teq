// An export's forwarder of a case class's companion applied to no arguments: the `apply` inserted
// after it is the companion's synthesized one, the class's creation with its defaults; a plain
// class's constructor proxy, through the forwarder that names it.
object Core:
  case class WithFlags(clocks: Boolean = true, moves: Boolean = true):
    def both = clocks && moves
  class Plain(val n: Int = 4)

object Game:
  export Core.*

object Main:
  import Game.WithFlags
  def main(args: Array[String]): Unit =
    val f: WithFlags = WithFlags()
    val g = Game.WithFlags(moves = false)
    println(s"${f.both} ${g.both}")
    println(Game.WithFlags().copy(clocks = false))
    println(Game.Plain().n + Game.Plain(1).n)
