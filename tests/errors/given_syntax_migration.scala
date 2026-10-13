// teq: --source 3.0-migration
// Under `-source 3.0-migration` the keywords Scala 3 added are identifiers, as dotty's scanner
// reads them (`Scanners.toToken`, `scala3keywords`): a given is no definition there (scalac:
// "expression expected but : found"); without the setting the program is a given object.
// expect: 8:9: error:
trait Parent
object O {
  given t: Parent() with {
    def extra = "ok"
  }
}
@main def run() = println(O.t.extra)
