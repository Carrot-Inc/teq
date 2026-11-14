// early_param_kind_wrong's two files with their names exchanged: the class is typed first; the same two refusals.
// expect: B.scala:7:31: error: type argument List does not have the same kind as its bound [G[_]]
// expect: B.scala:8:47: error: type argument H does not have the same kind as its bound [G[_]]
// expect: 2 errors found
package earlykindwrong

class WrongKind extends LKind[List]
class ParamKind[H[X] <: AnyRef] extends LKind[H]
