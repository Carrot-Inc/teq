// expect: right-hand side
// expect: 4 errors found
object Missing:
  opaque type Plain
  opaque type Upper <: Int
  opaque type Generic[A]
  opaque type GenericUpper[A] <: List[A]
