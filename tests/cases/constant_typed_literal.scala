// A `final val` ascribed a literal type is a constant, as one without a type is: a use is its
// literal, which leaves the object uninitialised (scalac folds both), a lazy one's too, and a use
// typed before the val's initialiser.
object Early:
  def read: Int = C.T + C.Z
object C:
  println("C initialised")
  final val T: 1 = 1
  final lazy val Z: 4 = 4
  final val S: "s" = "s"
  final val K = 2
@main def run(): Unit =
  println(Early.read)
  println(C.T + C.K)
  println(C.S)
