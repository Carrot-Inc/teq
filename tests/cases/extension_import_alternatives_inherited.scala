// Two imports of one scope that each bring an extension `value`, the same member of a class both
// objects extend: dotty's `tryExtension` tries each import (`findRef`'s `altImports`), the nearest
// one's failure included, each seen from its own object. Inside the opaque types' scope the
// implicit scope of `UserMarks` is its underlying list's (`collectParts`), so only the outer
// import's `value` reaches it (lila's `User.toString`).
object newtypes:
  abstract class TotalWrapper[Newtype, Impl](using Newtype =:= Impl):
    inline def apply(inline s: Impl): Newtype = s.asInstanceOf[Newtype]
    extension (inline a: Newtype) inline def value: Impl = a.asInstanceOf[Impl]
  abstract class YesNo[A](using A =:= Boolean) extends TotalWrapper[A, Boolean]:
    extension (inline a: A) inline def yes: Boolean = a.asInstanceOf[Boolean]
import newtypes.*
// The same without an opaque type: master's teq tried the nearest import alone.
abstract class Shows[T](tag: String):
  extension (t: T) def shown: String = tag + t
object IntShows extends Shows[Int]("i")
object StrShows extends Shows[String]("s")
object user:
  case class User(marks: UserMarks, enabled: UserEnabled):
    import user.UserMarks.*
    import user.UserEnabled.*
    override def toString = s"User ${marks.value.mkString(", ")} ${enabled.value} ${enabled.yes}"
    def dirty = marks.dirty
  enum UserMark:
    case boost, engine
  opaque type UserMarks = List[UserMark]
  object UserMarks extends TotalWrapper[UserMarks, List[UserMark]]:
    extension (a: UserMarks) def dirty: Boolean = a.value.nonEmpty
  opaque type UserEnabled = Boolean
  object UserEnabled extends YesNo[UserEnabled]
@main def run(): Unit =
  val u = user.User(user.UserMarks(List(user.UserMark.boost, user.UserMark.engine)), user.UserEnabled(true))
  println(u)
  println(u.dirty)
  import user.UserMarks.*
  import user.UserEnabled.*
  println(u.marks.value.size + " " + u.enabled.value)
  import IntShows.*
  import StrShows.*
  println("a".shown + " " + 1.shown)
