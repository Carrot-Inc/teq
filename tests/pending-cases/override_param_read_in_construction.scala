// An overriding val parameter read through a member while its enum is constructed:
// the case's field (`t` is 1), as Scala writes a whole chain's parameter fields before
// the first body; teq's JVM output is to read it too. The direct read (`t0`, dotty's
// `Constructors.intoConstr`) is tests/cases/override_param_read_in_construction.scala's.
enum H(val s: Int):
  def f = s
  val t = f
  case K(override val s: Int) extends H(s + 1)

@main def run(): Unit =
  val k = H.K(1)
  println(s"${k.s} ${k.t}")
