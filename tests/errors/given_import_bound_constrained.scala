// A `given T` selector admits a generic given where an instance of it within the given's own
// bounds matches `T` (`matchesImportBound` on the opened signature): `box[A <: String]` has none
// that is a `Box[Int]`, so nothing is imported (scalac's E172).
trait Box[A] { def value: Int }
object Lib {
  given box[A <: String]: Box[A] with { def value = 1 }
}
object Main {
  import Lib.{given Box[Int]}
  def main(args: Array[String]): Unit = println(summon[Box[String]].value)
}

// expect: given_import_bound_constrained.scala:10:68: error: no given instance of type Box[String] was found
