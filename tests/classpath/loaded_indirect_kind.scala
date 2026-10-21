// jars: fixtures
// A client of scalac's pickles of a class and an opaque type whose parameter a constructor bounds indirectly
// (`IndC[G <: IndP#F]`, `IndLib.IndO[G <: IndP#F]`, tests/tasty/src/indirect.scala): the parameter's information is
// read for the kind query whatever its arity (`ClassDenotation.typeParams`; an opaque type's definition's lambda), so
// each is refused where a `F[_]` is wanted, as scalac 3.8.4 refuses them (`Type argument fix.indirect.IndC does not have
// the same kind as its bound [_$1]`; the reviews of `b8eb8842a` and `4fd7f3293`).
// expect: 10:16: error: type argument IndC does not have the same kind as its bound [_]
// expect: 11:17: error: type argument IndO does not have the same kind as its bound [_]
class Box[F[_]]
type Bad = Box[fix.indirect.IndC]
type BadO = Box[fix.indirect.IndLib.IndO]
