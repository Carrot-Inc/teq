// A conversion given in an object nested in a trait, the object a type alias's function type
// names (`type ReadPref = ReadPref.type => Pref`, lila's `dsl`): through the object extending the
// trait, `dsl.ReadPref`'s implicit scope holds `dsl.ReadPref`'s givens, its prefix kept.
import scala.language.implicitConversions
class Pref(val n: Int)
trait Lang:
  type ReadPref = ReadPref.type => Pref
  object ReadPref:
    given Conversion[ReadPref, Pref] = f => f(this)
object dsl extends Lang
@main def run(): Unit =
  val f: dsl.ReadPref = _ => new Pref(3)
  val p: Pref = f
  println(p.n)
