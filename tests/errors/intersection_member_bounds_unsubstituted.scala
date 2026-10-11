// expect: 11:39: error: type argument C[Serializable & Product] & D[Serializable & Product] does not conform to upper bound C[Serializable & Product] & D[X]
// expect: 1 error found
// Two polymorphic members met with their bounds intersected as the tag's `infoMeet` intersects
// them, the second's bounds not read in the first's binders (`Denotations.scala` 575): the joint
// `U`'s bound is `C[T] & D[X]`, `X` the second method's own parameter, which no type argument
// fits beside `C[..]`, as scalac reports.
trait C[+T]
trait D[+T]
trait A { def f[T <: Serializable, U <: C[T]](u: List[U]): String }
trait B { def f[X <: Product, Y <: D[X]](u: List[Y]): String }
def unsubstituted(r: A & B): String = r.f[Serializable & Product, C[Serializable & Product] & D[Serializable & Product]](Nil)
