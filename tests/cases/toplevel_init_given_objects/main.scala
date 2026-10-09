// The shapes where teq's JVM output departs from scalac's: a folded `final val` and a given object
// initialise nothing, and a given with parameters and a body initialises its file, as scalac's
// `<file>$package` makes it through a def.
@main def main(): Unit =
  println("final val"); println(nfinal.x)
  println("given object")
  locally:
    import ngo.given
    println(summon[ngo.Tag].x)
  println("param given object")
  locally:
    import tpgo.given
    given Int = 3
    println(summon[tpgo.Tag].x)
  println("end")
