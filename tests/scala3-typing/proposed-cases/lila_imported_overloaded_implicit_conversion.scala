// lila: tv/Env.scala:35 (`import org.apache.pekko.pattern.ask` then `actor ? msg`; pekko's
// AskSupport has `implicit def ask(actorRef: ActorRef)` and a non-implicit overload `ask(ref, msg)`)
// Importing an overloaded name makes its implicit alternative an eligible conversion.
import scala.language.implicitConversions

class ActorRef(val name: String)
final class AskableActorRef(ref: ActorRef):
  def ?(msg: Any): String = s"${ref.name} ? $msg"

trait AskSupport:
  implicit def ask(actorRef: ActorRef): AskableActorRef = AskableActorRef(actorRef)
  def ask(actorRef: ActorRef, message: Any): String = AskableActorRef(actorRef) ? message
object pattern extends AskSupport

@main def run() =
  import pattern.ask
  val actor = ActorRef("tv")
  println(actor ? "Connect")
