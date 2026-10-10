// expect: 7:19: error: type NoSuchType not found
// expect: 1 error found
// A `given T` selector's bound is typed with the import (`Namer.importBound`): one that does not
// resolve is reported, never an import of every given.
object G { given Int = 1 }
@main def run(): Unit =
  import G.{given NoSuchType}
  println(summon[Int])
