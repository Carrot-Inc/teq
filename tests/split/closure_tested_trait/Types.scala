// A case class the program does not reach until an edit summons its mirror: its companion's
// `fromProduct` casts the union's element to `java.io.Serializable`, the trait Main.scala's thunks
// carry (tests/split-watch.sh).
enum Kind(val entryName: String):
  case Plain extends Kind("plain")
  case Fancy extends Kind("fancy")

final case class Other(value: String)

final case class Unreached(name: String, kind: Kind | Other, flag: Boolean)
