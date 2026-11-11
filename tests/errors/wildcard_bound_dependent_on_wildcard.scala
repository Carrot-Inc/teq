// A parameter's bound naming another parameter whose argument is a wildcard sees that argument as the skolem
// `<app>#A` (`Checking.checkAppliedType`'s `paramInfoAsSeenFrom(tree.tpe)`, Checking.scala 118; `argForParam`'s
// `TypeRef(pre, tparam)`, Types.scala 2794), bounded by the wildcard: below it only what is below its lower end,
// above it only what is above its upper end; a covariant parameter's with no lower bound the alias of its upper end
// (`argDenot`, Types.scala 2597). Both reject each line and show the skolem (E057):
//   13:22 Type argument String does not conform to upper bound C[? >: Int, String]#A
//   15:22 Type argument String does not conform to lower bound L[? <: Int, String]#A
//   17:16 Type argument String does not conform to upper bound EI[?, String]#T
//   18:22 Type argument  >: Int does not overlap with upper bound C[? <: Int, ? >: Int]#A
//   20:12 Type argument  >: Int does not overlap with upper bound W[? >: Int, ? <: AnyVal]#B
//   22:23 Type argument String does not conform to upper bound Co[? <: Int, String]#A
class C[A, B <: A]
type X1 = C[? >: Int, String]
class L[A, B >: A]
type X2 = L[? <: Int, String]
class EI[T <: String, U <: T]
type X3 = EI[?, String]
type X4 = C[? <: Int, ? >: Int]
class W[A <: B, B]
type X5 = W[? >: Int, ? <: AnyVal]
class Co[+A, B <: A]
type X6 = Co[? <: Int, String]
// expect: 13:23: error: type argument String does not conform to upper bound C[? >: Int, String]#A
// expect: 15:23: error: type argument String does not conform to lower bound L[? <: Int, String]#A
// expect: 17:17: error: type argument String does not conform to upper bound EI[?, String]#T
// expect: 18:23: error: type argument ? >: Int does not overlap with upper bound C[? <: Int, ? >: Int]#A
// expect: 20:13: error: type argument ? >: Int does not overlap with upper bound W[? >: Int, ? <: AnyVal]#B
// expect: 22:24: error: type argument String does not conform to upper bound Co[? <: Int, String]#A
// expect: 6 errors found
