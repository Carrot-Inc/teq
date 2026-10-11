// A hole of an interpolation is an argument of `StringContext.s(args: Any*)`: a value class's null, a generic
// result's, renders as `null` with two holes too, the temporary the evaluation order takes holding the reference it
// was made (dotty's `Boxing.adaptToType` passes a tree that conforms as it is; master's JVM printed `0:0`).
class V(val x: Int) extends AnyVal
class W(val s: String) extends AnyVal
def missing[A]: A = null.asInstanceOf[A]
@main def run() =
  println(s"${missing[V]}:${missing[V]}")
  println(s"${missing[W]}:${missing[V]}")
  println(s"${new V(3)}:${new W("w")}")
  val v = new V(4)
  println(s"${v.x}:${missing[V]}:$v")
  println(s"${missing[V]}")
