// A given of a local object, imported in the block that defines it (dotty's import of a local
// module's givens): the search finds it through the import.
object Lib:
  def result: Int =
    object O:
      given x: Int = 42
    import O.given
    summon[Int]
@main def run(): Unit = println(Lib.result)
