// expect: 12:16: error: illegal reference to type parameter A from enum case
// expect: 12:18: error: Not found: type F
// expect: 13:16: error: illegal reference to type parameter F from enum case
// expect: 13:18: error: Not found: type A
// absent: 12:16: error: Not found: type A
// absent: 13:16: error: Not found: type F
// `typeParamIsReferenced`'s search folds an infix type (`A F Int`, dotty's `InfixOp`) as its left operand, its
// operator, then its right operand (`foldMoreCases`), so the first reference is `A`'s; an applied type (`F[A, Int]`)
// its constructor first. scalac's reporter hides the not-found error at the place the illegal reference took
// (`UniqueMessagePositions`).
enum E[A, F[_, _]]:
  case C[X](x: A F Int) extends E[Int, Tuple2]
  case D[X](x: F[A, Int]) extends E[Int, Tuple2]
