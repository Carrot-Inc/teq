// A value export's forwarders of a name the class defines join its own alternatives, as lila's
// `Game` exports `players.apply as player` beside its own `player` and `player[U](user: U)`; of
// the alternatives none is most specific for `player(false)` but the one whose result takes no
// further arguments (dotty's resolveOverloaded1, noCurried).
case class ByColor[A](white: A, black: A):
  def apply(c: Boolean): A = if c then white else black
  def apply[B](c: Boolean)(f: A => B): B = f(apply(c))

case class Game(players: ByColor[String]):
  export players.{ white as whitePlayer, apply as player }
  def player: String = players(true)
  def player[U](user: U)(using Ordering[U]): Option[String] = None

@main def run(): Unit =
  val g = Game(ByColor("w", "b"))
  println(g.player)
  println(g.player(false))
  println(g.player(true)(_.length))
  println(g.player(3))
  println(g.whitePlayer)
