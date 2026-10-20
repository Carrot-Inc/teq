// lila: appeal/Appeal.scala:85 (`extension (appeals: UserAppeals) def get = appeals.get`)
// `x.get` selects the receiver's own member `get` (then eta-expands it); an extension
// method of the same name is not tried, so there is no recursion.
object Appeals:
  opaque type UserAppeals = Map[String, Int]
  object UserAppeals:
    def apply(m: Map[String, Int]): UserAppeals = m
    extension (appeals: UserAppeals)
      def muted = appeals.values.exists(_ > 0)
      def get = appeals.get

@main def run() =
  import Appeals.UserAppeals
  val a = UserAppeals(Map("x" -> 1))
  println(a.get("x"))
  println(a.muted)
