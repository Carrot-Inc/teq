// Of alternatives none of which is most specific, those whose result takes no further arguments are
// taken of every alternative, and the overload resolved again among them: `player(false)` is the
// first, which beats the polymorphic `player[U]` it then meets alone (dotty's resolveOverloaded1,
// noCurried).
class Game(white: String, black: String):
  def player(c: Boolean): String = if c then white else black
  def player[B](c: Boolean)(f: String => B): B = f(player(c))
  def player: String = white
  def player[U](user: U)(using Ordering[U]): Option[String] = None

@main def run(): Unit =
  val g = Game("w", "b")
  println(g.player)
  println(g.player(false))
  println(g.player(true)(_.length))
  println(g.player(3))
