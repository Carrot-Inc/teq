package lnkoka

// A parameter whose own clause has a kind of its own (`F[G[_]]`) and a member declared so: read from the products, the
// kinds of their parameters are kept (the loader's lambda bound, `typeParams`' parameter infos), so a client passing a
// constructor of that kind is accepted over the products as over the sources (`loadednestedkind` the refusals).
class AnnFG[F[G[_]]]
class AnnF[F[_]]
trait N:
  type F[G[_]]
