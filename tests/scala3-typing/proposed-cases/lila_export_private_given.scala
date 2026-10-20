// lila: user.UserRepo { export lila.user.BSONHandlers.given; ... updateField(.., HashedPassword(..)) }
// where BSONHandlers declares `private[user] given BSONHandler[HashedPassword]`.
// The qualified-private given is accessible at the export inside package user, so the export
// creates a given forwarder for it as well.
package lila.user

trait Writer[A] { def write(a: A): String }
case class Hashed(bytes: Array[Byte])

object Handlers:
  private[user] given Writer[Hashed] = h => s"hashed(${h.bytes.length})"
  given Writer[Int] = _.toString

def updateField[V: Writer](field: String, v: V): String = s"$field=${summon[Writer[V]].write(v)}"

final class Repo:
  export lila.user.Handlers.given
  def blankPassword: String = updateField("bpass", Hashed(Array.empty))
  def count: String = updateField("count", 3)

@main def run() =
  println(Repo().blankPassword)
  println(Repo().count)
