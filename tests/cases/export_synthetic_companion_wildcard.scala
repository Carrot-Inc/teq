// A wildcard export of a case class's synthetic companion forwards nothing: its members are
// synthetic, which a wildcard passes over (dotty's `addWildcardForwarders`,
// `excluded = PrivateOrSynthetic`), so the clause is no error and names nothing.
object user:
  case class Plan(months: Int)
export user.Plan.*
object pkgs:
  case class Tier(level: Int)
  export pkgs.Tier.{given, *}
@main def run(): Unit =
  println(user.Plan(1))
  println(pkgs.Tier(2).level)
