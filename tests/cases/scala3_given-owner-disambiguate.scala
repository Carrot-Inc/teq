// Adapted from scala3 tests/pos/given-owner-disambiguate.scala and the first scheme of tests/pos/given-priority.scala (Apache-2.0, see tests/scala3/README.md); class inheritance replaced by traits, the results printed.
//> using scala 3.8.4
trait General { def name: String }
trait Specific extends General
class GI extends General { def name = "General" }
class SI extends Specific { def name = "Specific" }
trait LowPriority:
  given a: General = GI()
object NormalPriority extends LowPriority:
  given b: Specific = SI()
@main def main(): Unit =
  import NormalPriority.given
  val x = summon[General]
  val y: Specific = x
  println(x.name)
  println(summon[Specific].name)
