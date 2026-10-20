// lila: round.JsonView: `import withFlags.me` then userJsonView.roundPlayer(user, p)
// with case class ExportOptions(...)(using val me: Option[Me]) and roundPlayer(...)(using Option[Me]).
// A `using val` class parameter is a given member; a named import of it brings it into
// implicit scope (ImportInfo.importedImplicits, named selectors: implicitMembersNamed).
class Me(val name: String)
case class Opts(rating: Boolean = true)(using val me: Option[Me])

def roundPlayer(u: String)(using me: Option[Me]): String = s"$u seen by ${me.map(_.name)}"

def json(opts: Opts): String =
  import opts.me
  roundPlayer("bob")

@main def run() =
  println(json(Opts()(using Some(Me("alice")))))
  println(json(Opts()(using None)))
