// An export of an object's givens implements a parent's abstract given: the forwarder is a member
// of the class, so the search finds one candidate (dotty's `Namer.exportForwarders`, the forwarder
// entered as a member that implements the deferred one).
trait Reader[A]:
  def name: String
class User
object Handlers:
  given userHandler: Reader[User] = new Reader[User] { def name = "handler" }
abstract class Base:
  given userHandler: Reader[User]
class Repo extends Base:
  export Handlers.given
  val result = summon[Reader[User]]
@main def main() = println(new Repo().result.name)
