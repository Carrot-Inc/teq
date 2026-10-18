// teq: --source 3.7
// A tuple pattern past 22 elements is extracted by `TupleXXL.unapplySeq`, whose elements are `Any`s
// (`UnapplyArgs.argTypes`): a given bind's type, annotated `@unchecked` by `makeLambda` (`tests/warn/i23164.scala`),
// is refutable against it in a generator checked as `IrrefutableGenFrom` (the review of 1888ac4df).
class T1[F[_]]
@main def Test =
  val xs: List[(Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, Int, T1[Option])] =
    List((1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, new T1[Option]))
  for (a1, a2, a3, a4, a5, a6, a7, a8, a9, a10, a11, a12, a13, a14, a15, a16, a17, a18, a19, a20, a21, a22, given T1[Option]) <- xs do println(a1 + a22)
// expect: 9:115: error: pattern's type T1[Option] is more specialized than the right hand side expression's type Any
