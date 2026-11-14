// A class applied inside the bound of its own higher-kinded parameter to an argument outside that bound; a
// parameter whose own bound's member has another class's type passed on; and a class whose member has another
// type: each refused where the argument stands. scalac 3.8.4 refuses the same three lines.
// expect: 9:35: error: type argument Bare does not conform to upper bound [X] =>> AnyRef{def self: Own[Bare]}
// expect: 12:59: error: type argument H does not conform to upper bound [X] =>> AnyRef{def self: Ring[H]}
// expect: 13:24: error: type argument Bare does not conform to upper bound [X] =>> AnyRef{def self: Ring[Bare]}
// expect: 3 errors found
class Bare[X] { def self: Int = 1 }
class Own[F[X] <: { def self: Own[Bare] }]
class Ring[F[X] <: { def self: Ring[F] }]
class Other[G[X] <: { def self: Other[G] }]
class Passed[H[X] <: { def self: Other[H] }] extends Ring[H]
class Use extends Ring[Bare]
