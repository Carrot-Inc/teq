package likok

// A class parameter bounded by a constructor indirectly (`G <: P#F`, no lambda in the pickle, its stored arity 0): read
// from the products, its information is loaded for a kind query whatever its arity (`ClassDenotation.typeParams`'
// parameter infos), so a client passing the class where a `G <: P#F` is wanted is accepted over the products as over the
// sources (`loadedindirectkind` the refusal; with `G[_]` scalac refuses its bounds, E057,
// `tests/scala3-typing/proposed-errors/hk_argument_param_bounds.scala`).
trait P:
  type F[X]
class C[G <: P#F]
