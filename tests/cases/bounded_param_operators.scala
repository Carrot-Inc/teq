// A type parameter bounded by a primitive has the bound's operators on either side and as a prefix, as scalac finds
// them on the bound (`Int.+(x: Int)` taking a `T <: Int`): master took `1 + x`, `x * y`, `!x` and `-x` for no
// operator of `T` ("operator + cannot be applied to Int and T", "value unary_! is not a member of T").
def neg[T <: Boolean](x: T): Boolean = !x
def plus[T <: Int](x: T): Int = 1 + x
def plus2[T <: Int](x: T): Int = x + 1
def both[T <: Int](x: T, y: T): Int = x * y
def lt[T <: Double](x: T): Boolean = x < 2.5
def str[T <: String](x: T): String = x + "!"
def cmp[T <: Long](x: T): Long = -x
@main def run() =
  println(neg(true))
  println(plus(2))
  println(plus2(2))
  println(both(3, 4))
  println(lt(1.0))
  println(str("a"))
  println(cmp(5L))
