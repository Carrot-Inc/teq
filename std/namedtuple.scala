package scala

// Named tuples (Scala 3.7): `(name: T, ...)` is `NamedTuple[("name", ...), (T, ...)]`, the tuple
// of the values at run time on every target. The compiler supplies the literal, selection by
// name, the pattern and the conformance to and from the plain tuple (`src/typer/namedtuple.rs`).
object NamedTuple:
  opaque type AnyNamedTuple = Any
  opaque type NamedTuple[N <: Tuple, +V <: Tuple] <: AnyNamedTuple = V

  extension [N <: Tuple, V <: Tuple](x: NamedTuple[N, V])
    def toTuple: V = x
