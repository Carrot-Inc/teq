// An object an export forwards, `final def Challenger: hub.Challenge.Challenger.type`, is a stable path
// to it: a type and a term selected through it are the object's (dotty's StableRealizable forwarder;
// lila's `Challenge.Challenger.Registered` and `Challenger.Anonymous.apply` through `export hub.Challenge.*`).
package xcore
object hub:
  object Challenge:
    case class Rating(int: Int)
    enum Challenger:
      case Registered(id: String, rating: Rating)
      case Anonymous(secret: String)
      case Open

object Challenge:
  export hub.Challenge.*

case class Ch(challenger: Challenge.Challenger, destUser: Option[Challenge.Challenger.Registered])

@main def run(): Unit =
  import Challenge.*
  val c = Ch(Challenger.Open, Some(Challenger.Registered("u", Rating(3))))
  println(c)
  println(Some("s").map(Challenger.Anonymous.apply))
