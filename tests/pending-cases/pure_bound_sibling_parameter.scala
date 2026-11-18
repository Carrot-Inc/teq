// std: scala-library
// teq: --std=scala-library
// A method's type parameter bounded by a sibling parameter bounded by `caps.Pure` has `Pure` as its bound's type
// symbol (a parameter of the clause is a proxy of its bound), so `boundsViolations`' `dropPure` drops it and the
// argument `Int` is held to no bound: scalac builds it. teq holds `Int` to `Pure`: dropping it by the bound's type
// symbol needs an alias's identity, which teq's types do not keep.
import caps.Pure

final class Token extends Pure { def value: Int = 7 }

def f[A <: Pure, B <: A](b: B): A = b

// A written application's bounds are its parameters' as instantiated (`checkAppliedType`): `B`'s is `Pure`, dropped.
class C[A, B <: A]
type X = C[Pure, Int]

@main def run(): Unit = println(f[Token, Int](1))
