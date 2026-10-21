// A definition's type parameters, the namer's (a class's, a method's, an alias's `type N[..]`): their completion
// and `checkNonCyclic`, and their annotations completed as a binder's are (`Namer.addAnnotations`,
// `completeAnnotations`), scalac 3.8.4's eight errors; teq reports none. A part of its own (the table's decision 1b of
// the fourth fix-up), with the annotations of definitions.
// expect: 14:9: error: illegal cyclic type reference: upper bound C of type B refers back to the type itself
// expect: 15:7: error: illegal cyclic type reference: lower bound C of type B refers back to the type itself
// expect: 16:8: error: illegal cyclic type reference: upper bound C of type B refers back to the type itself
// expect: 17:21: error: Cyclic reference involving type B
// expect: 18:20: error: illegal cyclic type reference: upper bound B of type C refers back to the type itself
// expect: 19:9: error: main annotation cannot be applied to type B
// expect: 20:13: error: not found: type C
// expect: 20:24: error: not found: type B
class ann[T] extends scala.annotation.StaticAnnotation
class K[B <: C, C <: B]
def m[B >: C, C >: B] = 1
type N[B <: C, C <: B] = Int
type N1[@ann[C] B, @ann[B] C] = Int
type N2[@ann[C] B, C <: B] = Int
type N3[@main B] = Int
def m1[@ann[C] B, @ann[B] C] = 1
