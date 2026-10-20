// lila: security/GarbageCollector.scala:77 (`alts.otherUsers.sortByReverse(_.atInstant)`, with
// `object OtherUser: given [U: AtInstant] => AtInstant[OtherUser[U]]`)
// `x.atInstant` finds the extension in a given of the implicit scope even when that given is
// conditional (type parameter + context bound), resolving its own using clause.
trait AtInstant[A]:
  def apply(a: A): Long
  extension (a: A) def atInstant: Long = apply(a)

case class User(createdAt: Long)
object User:
  given AtInstant[User] = _.createdAt

case class OtherUser[U](user: U, ips: Set[String])
object OtherUser:
  given [U: AtInstant] => AtInstant[OtherUser[U]] = _.user.atInstant

@main def run() =
  val alts = List(OtherUser(User(7), Set()), OtherUser(User(3), Set()))
  println(alts.sortBy(_.atInstant).map(_.user))
