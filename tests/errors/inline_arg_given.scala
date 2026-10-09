// expect: 8:50: error: no given instance of type O was found for parameter x
// expect: 1 error found
// A using parameter of an inline body is a given of its declared type: `summon[O.type]` over
// `x: B` finds nothing, as scalac finds nothing at the definition, whatever the argument, and
// reports it there, once.
trait B
object O extends B
inline def f(using x: B): String = summon[O.type].toString
@main def run(): Unit = println(f(using O))
