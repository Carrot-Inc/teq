// jars: scala-library tapir-core-jvm magnolia123-jvm sttp-model-core-jvm sttp-shared-core-jvm sttp-shared-ws-jvm
// A jar's case class created where the build converted its companion for a macro's run: `Schema.schemaForMap`, a
// macro, converts tapir's `Schema`, whose companion runs code; each `derives Schema` creates `new Schema` in its
// derivation's `withCache` with that companion untouched, as a build that runs no such macro writes it (one over
// the products of an upstream holding the `derives`). The program's own companion that runs code is touched before
// `new`, as scalac's `apply` loads it first.
package jct
import java.time.LocalDate
import sttp.tapir.Schema
final case class Rec(a: Int, b: Option[String]) derives Schema
final case class Own(n: Int)
object Own:
  val made: String = "own"
object Use:
  given Schema[Map[LocalDate, Int]] = Schema.schemaForMap[LocalDate, Int](_.toString)
  def own(): Own = Own(1)
@main def run(): Unit =
  println(summon[Schema[Rec]].name.map(_.fullName))
  println(Use.own())
