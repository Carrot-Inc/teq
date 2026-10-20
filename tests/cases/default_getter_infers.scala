// A default's getter has a type of its own (`Namer.inferredResultType`'s `rhsType`): the parameter's type
// approximated is no `T`, so the getter's type is the default's, `String`, which the call's argument has and which
// fixes `T` (`Applications.tryDefault`).
object G:
  def g[T](x: T = "abc"): T = x
  def h(x: Long = 1): Long = x
  val z: String = g()
@main def run(): Unit = println(G.z + " " + G.h())
