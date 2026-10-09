// A union of singleton types is a `Singleton` bound's argument, as under scalac.
class C[T <: Singleton](val name: String)
type D = C[1 | 2]

@main def Main(): Unit =
  val d: D = C[1 | 2]("d")
  println(d.name)
