package lik

// A class parameter bounded by a constructor indirectly (`G <: P#F`, no lambda in the pickle, its stored arity 0): read
// from the products, its information is loaded for a kind query whatever its arity (`ClassDenotation.typeParams`'
// parameter infos), so a client passing the class where a `F[_]` is wanted is refused over the products as over the
// sources, by teq and by scalac (the review of `b8eb8842a`, item 1; `loadedindirectkindok` the accepted control).
trait P:
  type F[X]
class C[G <: P#F]
