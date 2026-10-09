// An intersection's member declared with bounds on both sides has the bounds met, `<: AnyRef & String`,
// as scalac's `&` of two `TypeBounds`.
trait L { type T <: AnyRef }; trait R { type T <: String }
def f(x: L & R)(y: x.T): String = y
@main def run(): Unit = println("ok")
