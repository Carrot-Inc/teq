// A block's class whose higher-kinded parameter has a written bound, applied before its definition to an argument
// of another kind: refused at one worker as at several. scalac 3.8.4: "Type argument List does not have the same
// kind as its bound [G[_$1]] <: AnyRef".
// expect: 7:32: error: type argument List does not have the same kind as its bound [G[_]]
// expect: 1 error found
def blockKind = {
  class Applied extends KLater[List]
  class KLater[F[G[_]] <: AnyRef] { def value = 302 }
  new Applied().value
}
