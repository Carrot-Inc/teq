// A type variable bounded below by a stable path's type keeps the path itself as its bound (dotty's `TermRef`),
// widened when the variable is instantiated to the path's declared type with its alias
// (`ConstraintHandling.widenInferred`'s `widenSingletons`); a bound that is an alias of a union, from an ascription
// or an application's result, meets the variable as the union's members (`TypeComparer.compareTypeParamRef`:
// `tp1.dealias` an `OrType` recurs on it before `addConstraint`), and the members of the soft union of the bounds
// are each widened before they are joined (`widenInferred`'s `widenUnion`). scalac infers `String` for
// `one((a: AssetType))` (a `Typed` tree is no `TermRef`) and for `mkTuple(() => (a, 1))` (`K` bounded by `a.type`
// through `Tuple2.apply`'s `T1`, then by the constants of the dealiased alias when the tuple's type meets
// `(K, V)`), and `AssetType` for `one(a)`; teq records the path's bound as its widened type with the alias, which
// the second comparison finds recorded and adds nothing to, and reads the ascription as the path it ascribes
// (type mismatch: found Inv[AssetType], required Inv[String]).
type AssetType = "sound" | "image" | "book" | "net"
class Inv[K](val k: K)
def one[K](x: K): Inv[K] = Inv(x)
def mkTuple[K, V](f: () => (K, V)): Inv[K] = Inv(f()._1)
val a: AssetType = "sound"
val i1 = mkTuple(() => (a, 1)); val c1: Inv[String] = i1
val i2 = one((a: AssetType)); val c2: Inv[String] = i2
val i3 = one(a); val c3: Inv[AssetType] = i3
@main def main(): Unit = println("ok")
