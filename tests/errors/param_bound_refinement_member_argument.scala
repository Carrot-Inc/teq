// A higher-kinded parameter passed on to a class whose own parameter's bound asks a member the parameter's bound
// does not give: another result, a `def` where a `val` is asked, another name, a parameter list where none is; and
// a class with no such member; a parameter whose bound reaches such a bound through an intersection; and a `var`
// passed where a `val` is asked, a `val` where a `var` is. scalac 3.8.4 refuses the same eight lines.
// expect: 17:63: error: type argument G does not conform to upper bound [X] =>> AnyRef{def self: Int}
// expect: 18:61: error: type argument G does not conform to upper bound [X] =>> AnyRef{val self: Int}
// expect: 19:59: error: type argument G does not conform to upper bound [X] =>> AnyRef{def self: Int}
// expect: 20:59: error: type argument G does not conform to upper bound [X] =>> AnyRef{def self(n: Int): Int}
// expect: 22:25: error: type argument Bare does not conform to upper bound [X] =>> AnyRef{def self: Int}
// expect: 24:81: error: type argument H does not conform to upper bound [X] =>> AnyRef{def self: Int}
// expect: 26:61: error: type argument G does not conform to upper bound [X] =>> AnyRef{val self: Int}
// expect: 27:61: error: type argument G does not conform to upper bound [X] =>> AnyRef{val self: Int}
// expect: 8 errors found
class Asked[F[X] <: { def self: Int }]
class AskedVal[F[X] <: { val self: Int }]
class AskedList[F[X] <: { def self(n: Int): Int }]
class OtherResult[G[X] <: { def self: String }] extends Asked[G]
class DefForVal[G[X] <: { def self: Int }] extends AskedVal[G]
class OtherName[G[X] <: { def other: Int }] extends Asked[G]
class NoList[G[X] <: { def self: Int }] extends AskedList[G]
class Bare[X]
class Use extends Asked[Bare]
trait Named
class InSides[G[X] <: { def self: String }, H[X] <: G[X] & Named] extends Asked[H]
class AskedVar[F[X] <: { var self: Int }]
class VarForVal[G[X] <: { var self: Int }] extends AskedVal[G]
class ValForVar[G[X] <: { val self: Int }] extends AskedVar[G]
