// A conversion given in an object nested in a trait converts through an object extending the trait:
// the implicit scope of `dsl.ReadPref` is seen from `dsl` (lila's `lila.db.dsl`), the typed placeholder
// `(_: dsl.ReadPref.type)` the function the conversion takes (Parsers.isWildcard).
import scala.language.implicitConversions
trait Dsl:
  enum Pref:
    case Primary
  type ReadPref = ReadPref.type => Pref
  object ReadPref:
    val primary: Pref = Pref.Primary
    given Conversion[ReadPref, Pref] = _(this)
object dsl extends Dsl
val p: dsl.Pref = (_: dsl.ReadPref.type).primary
@main def m(): Unit = println(p)
