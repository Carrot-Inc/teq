// A type alias of a trait naming an object of the trait (`type ReadPref = ReadPref.type => Pref`, lila's
// `lila.db.dsl`), read through an import of the object extending the trait: the check of whether
// completing an imported object reaches a class under completion asks once, not again through the
// imports it meets.
class Pref(val name: String)

trait dsl:
  type ReadPref = ReadPref.type => Pref
  object ReadPref:
    val pri: Pref = Pref("primary")

object dsl extends dsl
import dsl.*

@main def run(): Unit =
  val r: ReadPref = _.pri
  println(r(ReadPref).name)
