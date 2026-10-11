// expect: 13:26: error: Ambiguous overload. The overloaded alternatives of method f in trait A with types
// expect:  (x: List[Int]): Any
// expect:  (x: List[String]): Any
// expect: both match arguments (Nil)
// expect: 17:47: error: there is no parameter named x
// Two declarations whose erased signatures match and whose parameters do not (`List[Int]` and
// `List[String]`): neither one's information overrides the other's and they do not meet
// (`infoMeet`'s `NoType`), so both stay alternatives (`mergeSingleDenot`'s `MultiDenotation`) and
// `Nil` is ambiguous between them. Two parameters of different names meet under a synthetic name
// (`mergeParamNames`), which a named argument does not take.
trait A { def f(x: List[Int]): Any }
trait B { def f(x: List[String]): Any }
def bad(r: A & B): Any = r.f(Nil)
trait C { def g(x: Int): Product = ??? }
def ok(r: A & C): Any = r.f(List(1))
trait D { def g(y: Int): Serializable }
def named(r: C & D): Product & Serializable = r.g(x = 1)
def positional(r: C & D): Product & Serializable = r.g(1)
