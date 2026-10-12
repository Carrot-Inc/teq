// An eta-expansion's parameters take the expected function's parameter types where it is fully
// defined, an argument converted to the method's where a conversion applies (dotty's adapt1 gives
// etaExpand the arity, which leaves them to be inferred): lila's me.exists(of(f)) of an Option[Me]
// for an of(f)(user: User), a Conversion[Me, User] in scope.
case class User(name: String, enabled: Boolean)

object user:
  opaque type Me = User
  object Me:
    def apply(u: User): Me = u
    given Conversion[Me, User] = identity

import user.*

object Granter:
  def of(perm: String)(u: User): Boolean = u.enabled && perm.nonEmpty
  def of(perm: Int)(u: User): Boolean = u.enabled && perm > 0
  def opt(perm: String)(using me: Option[Me]): Boolean = me.exists(of(perm))

@main def run(): Unit =
  given Option[Me] = Some(Me(User("a", true)))
  println(Granter.opt("admin"))
  val check: Me => Boolean = Granter.of(1)
  println(check(Me(User("b", false))))
