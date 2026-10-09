// expect: +: (which is right-associative) and +! (which is left-associative) have same precedence and may not be mixed
// expect: +! (which is left-associative) and +: (which is right-associative) have same precedence and may not be mixed

infix type +:[A, B] = (A, B)
infix type +![A, B] = Either[A, B]

object Main:
  type RightFirst = Int +: String +! Boolean
  type LeftFirst = Int +! String +: Boolean
