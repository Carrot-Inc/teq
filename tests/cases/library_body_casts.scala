// jars: bodycasts-lib
// A jar's bodies that teq types again, as cats-effect's `Dispatcher` and `Resource.start` (tyrian's
// sandbox reaches both): the cast scalac makes of a pattern-bound wildcard's member to the method's
// type keeps the parts of its intersection that name no wildcard, and a local case class's defaults
// are its synthetic companion's getters.
@main def run(): Unit =
  println(bodycasts.Bodies.run[Option](Some(())))
  println(bodycasts.Bodies.start)
