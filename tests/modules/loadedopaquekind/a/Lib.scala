package lok

// An opaque type whose parameter a constructor bounds indirectly (`G <: P#F`): read from the products, its parameters
// are its definition's lambda's, their infos loaded for a kind query (`typeParams` of the opaque `TypeRef`), so a
// client passing it where a `F[_]` is wanted is refused over the products as over the sources, by teq and by scalac (the
// review of `4fd7f3293`, item 2).
trait P:
  type F[X]
object Lib:
  opaque type C[G <: P#F] = Any
