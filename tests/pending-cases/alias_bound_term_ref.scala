// A term reference that is no stable path (`ret`, a parameterless method; `(a, 1)._1`, a field of a value that is
// no path) is a `TermRef` under dotty (`Typer.typedIdent`, `typedSelect`), so a type variable it bounds widens to
// the reference's declared type with its alias kept (`ConstraintHandling.widenInferred`'s `widenSingletons`); teq
// types such a reference as its widened type and meets the variable with the alias's union dealiased, as it does
// for an application's result (type mismatch: found Inv[String], required Inv[AssetType]). Term references typed as
// singleton types throughout are another part's mechanism.
type AssetType = "sound" | "image" | "book" | "net"
class Inv[K](val k: K)
def one[K](x: K): Inv[K] = Inv(x)
val a: AssetType = "sound"
def ret: AssetType = a
val i1 = one(ret); val c1: Inv[AssetType] = i1
val i2 = one((a, 1)._1); val c2: Inv[AssetType] = i2
@main def main(): Unit = println("ok")
