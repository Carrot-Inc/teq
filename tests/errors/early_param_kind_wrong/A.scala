// A class whose higher-kinded parameter has a written bound, first applied in the file typed before the class's, to
// an argument of another kind: a class that takes a type, and another class's parameter that does. The parameter
// has its clause's kind from the start, so each is refused as it is where the class is typed first. scalac 3.8.4:
// "Type argument List does not have the same kind as its bound [G[_$1]] <: AnyRef", and the same of H.
// expect: A.scala:10:31: error: type argument List does not have the same kind as its bound [G[_]]
// expect: A.scala:11:47: error: type argument H does not have the same kind as its bound [G[_]]
// expect: 2 errors found
package earlykindwrong

class WrongKind extends LKind[List]
class ParamKind[H[X] <: AnyRef] extends LKind[H]
