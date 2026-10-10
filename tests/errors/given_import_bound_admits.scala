// A `given T` selector imports the givens matching `T` alone, whatever check runs: `summon[String]`
// finds none (scalac's E172).
object O { given int: Int = 1; given text: String = "bad" }
import O.{given Int}
@main def run(): Unit = println(summon[String])

// expect: given_import_bound_admits.scala:5:47: error: no given instance of type String was found
