// jars: scala-library chimney-jvm chimney-macro-commons-jvm scala-collection-compat-jvm
// std: scala-library
//> using dep io.scalaland::chimney:1.11.0
// chimney's enum-to-enum transformation in link mode: its macros read the enum's children
// through `Symbol.typeRef` (a case's val), cast `Ref(caseVal)` to the case's singleton type, and
// match on `Bind(x, Ref(caseVal))`, a stable identifier pattern.
import io.scalaland.chimney.dsl.*

object Source:
  enum Tier:
    case Credits
    case Cash
object Target:
  enum Tier:
    case Credits
    case Cash

object Main:
  def main(args: Array[String]): Unit =
    println(Source.Tier.Credits.transformInto[Target.Tier])
    println(Source.Tier.values.toList.map(_.transformInto[Target.Tier]))
