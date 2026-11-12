// `Checking.checkAppliedTypesIn` (Checking.scala 145-172) over the applications inside an
// inferred type, from PostTyper's `checkInferredWellFormed` (PostTyper.scala 281-289): a
// lambda's inferred parameter type `BS[?, IS]` and a value's inferred `List[BS[?, IS]]`
// violate `S <: BS[T, S]` through `skolemizeWildcardArgs`' skolem (`IS` is not below
// `BS[BS[?, IS]#T, IS]`), as the written `BS[?, IS]` does: scalac 3.8.4 rejects the
// first ("... in inferred type BS[?, IS]") and checks no inferred type after an error
// (alone, `v` gives "... in subpart BS[?, IS] of inferred type List[BS[?, IS]]").
trait BS[T, S <: BS[T, S]]
trait IS extends BS[Int, IS]
def g[S <: BS[?, S]](x: BS[?, S] => Int): Int = 6
def h[S <: BS[?, S]]: List[BS[?, S]] = Nil
val r = g[IS](_ => 0)
val v = h[IS]
