// An anonymous given's export of an object's overloaded method under the name of the parent's overloaded
// abstract `apply`: each alternative's forwarder, a member of the class, implements one (dotty's
// `Namer.exportForwarders`, every clause's forwarders entered as members; lila's `NewPlayer`).
trait NewPlayer:
  def apply(color: String, user: Option[Int]): String
  def apply(color: String, userId: Int, rating: Int): String
  def anon(color: String): String
object Player:
  def make(color: String, user: Option[Int]): String = s"$color $user"
  def make(color: String, userId: Int, rating: Int): String = s"$color $userId $rating"
  def makeAnon(color: String) = s"anon $color"
given newPlayer: NewPlayer = new:
  export Player.make as apply
  def anon(color: String) = Player.makeAnon(color)
@main def main() = println((summon[NewPlayer]("w", Some(1)), newPlayer("b", 2, 3), newPlayer.anon("x")))
