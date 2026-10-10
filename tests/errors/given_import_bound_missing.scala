// The type of a `given T` selector is resolved where the import stands: a missing one is an error
// (scalac's E006).
object O { given int: Int = 1 }
import O.{given Missing}
@main def run(): Unit = println(summon[Int])

// expect: given_import_bound_missing.scala:4:17: error: type Missing not found
