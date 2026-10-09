// jars: scala-library chimney chimney-macro-commons scala-collection-compat scala-java-time
//> using dep io.scalaland::chimney:1.11.0
// chimney 1.11.0's `transformInto` from its jar, its macros run in teq's interpreter: a case
// class into one with fewer fields, into one whose missing fields have defaults (default values
// enabled by a `transparent inline given` configuration, as chimney's docs write it for Scala 3),
// and into one with renamed fields through a `Transformer.define` given.
// scala-java-time is on the class path, as the application's is: chimney's macros time their
// derivation with `java.time.Instant.now()`, which the interpreter runs from that jar.
package chimneyinto

import io.scalaland.chimney.Transformer
import io.scalaland.chimney.dsl.*

final case class Source(id: Int, name: String, note: String, active: Boolean)
final case class Narrow(id: Int, name: String)
final case class WithDefaults(id: Int, name: String, score: Double = 1.5, tags: List[String] = Nil)
final case class Renamed(key: Int, label: String, active: Boolean)

object Renaming:
  given Transformer[Source, Renamed] =
    Transformer.define[Source, Renamed]
      .withFieldRenamed(_.id, _.key)
      .withFieldRenamed(_.name, _.label)
      .buildTransformer

def withDefaults(s: Source): WithDefaults =
  transparent inline given TransformerConfiguration[?] = TransformerConfiguration.default.enableDefaultValues
  s.transformInto[WithDefaults]

@main def main(): Unit =
  val s = Source(7, "seven", "unused", active = true)
  println(s.transformInto[Narrow])
  println(withDefaults(s))
  import Renaming.given
  println(s.transformInto[Renamed])
  println(List(s, s.copy(id = 8)).map(_.transformInto[Narrow]))
