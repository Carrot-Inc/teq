// teq: --source 3.8
// A generator's pattern that is no `IdPattern` is a case lambda's, checked (the review of 1888ac4df): `(K)` against
// its `x$1`, a literal at its constant type, an object at its module's, `null` at `Null`; the selector's path is
// reported unwidened for a singleton pattern type (`(x$1 : Int)`), widened for `Null`.
val K = 1
object O
@main def m =
  for (K) <- List(1, 1) do println("k")
  for 1 <- List(2) do ()
  for (O) <- List[Any](O) do ()
  for "a" <- List("b") do ()
  for null <- List("c") do ()
// expect: 8:8: error: pattern's type (K : Int) does not match the right hand side expression's type (x$1 : Int)
// expect: 9:7: error: pattern's type 1 does not match the right hand side expression's type (x$1 : Int)
// expect: 10:8: error: pattern's type O does not match the right hand side expression's type (x$1 : Any)
// expect: 11:7: error: pattern's type "a" does not match the right hand side expression's type (x$1 : String)
// expect: 12:7: error: pattern's type Null is more specialized than the right hand side expression's type String
