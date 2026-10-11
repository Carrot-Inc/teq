// Two generic members of an intersection whose results neither conforms to the other's are one
// member whose type parameters take both bounds (dotty's `infoMeet` of two PolyTypes,
// `paramInfos & paramInfos`, `TypeBounds.&`), met in the parts' order, as scalac 3.8.4 reports a
// type argument that meets one alone (E057).
trait P { def p: Int }
trait Q { def q: Int }
trait R1
trait R2
trait A { def value[T <: R1](x: Int): P }
trait B { def value[T <: R2](x: Int): Q }
def use(x: A & B): P & Q = x.value[String](1)
def rev(x: B & A): P & Q = x.value[String](1)
trait C { def value[T <: AnyRef](x: Int): P }
trait D { def value[T <: Product](x: Int): Q }
def both(x: C & D): P & Q = x.value[String](1)
// expect: 11:28: error: type argument String does not conform to upper bound R1 & R2
// expect: 12:28: error: type argument String does not conform to upper bound R2 & R1
// expect: 15:29: error: type argument String does not conform to upper bound AnyRef & Product
