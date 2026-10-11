// expect: 9:46: error: type mismatch: found Null, required Nothing
// expect: 1 error found
// A parameter of an invariant class of the parameter that two instances make a range
// (`R[Int] & R[String]`'s `put(x: Box[K])`): no `Box` fits the range where the parameter is
// written, so it takes `Nothing` (`ApproximatingTypeMap.derivedAppliedType`'s `range(Nothing, ..)`
// read at a parameter's variance) and `null` is refused, as scalac refuses it.
trait Box[T]
trait R[K] { def put(x: Box[K]): Unit }
def bad(r: R[Int] & R[String]): Unit = r.put(null)
