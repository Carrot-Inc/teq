// A clause's own parameter's bound is part of the bound an argument is held to: a class whose parameter takes a
// constructor of any subtype of Base is no argument for a clause that takes one of Sub's subtypes alone, nor is one
// that takes any type for a clause that takes Base's subtypes. scalac 3.8.4 refuses both (E057: "Type argument
// KBase does not conform to upper bound [G[X <: Sub]] =>> Any"). The class is first applied in the file typed
// before its own.
// expect: A.scala:12:29: error: type argument KBase does not conform to upper bound
// expect: A.scala:13:28: error: type argument KFree does not conform to upper bound
package earlykindinner

class KBase[F[X <: Base]]
class KFree[F[X]]
class Narrower extends LSub[KBase]
class Absent extends LBase[KFree]
