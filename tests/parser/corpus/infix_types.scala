import scala.compiletime.ops.int.*

infix type +:[A, B] = (A, B)
infix type +![A, B] = Either[A, B]
infix type **[A, B] = A => B
infix type Or[A, B] = Either[A, B]

object Main:
  val pair: Int *: Int *: EmptyTuple = (1, 2)
  val triple: Int *: String *: Boolean *: EmptyTuple = (1, "a", true)
  val sameType = summon[(Int *: String *: EmptyTuple) =:= (Int, String)]

  val right: Int +: String +: Boolean = (1, ("a", true))
  val left: Int +! String +! Boolean = Left(Right("s"))

  val product: 1 + 2 * 3 = 7
  val difference: 10 - 2 - 3 = 5
  val alnumBelowBar: Int Or String | Boolean = Right(true)
  val barBelowCons: Int *: EmptyTuple | String = "s"
  val stackTop: Int +! String ** Boolean +: Char = Right(((s: String) => s.isEmpty, 'c'))

  type ABC = 'A' | 'B' | 'C'
  val letter: ABC = 'B'
  val self: Singleton & this.type = this
  val negative: -1 | 2 = -1

  def prefixed[X <: Tuple](t: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int
    *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: Int *: X): Int = t.productArity

  def main(args: Array[String]): Unit =
    println(pair)
    println(triple)
    println(right)
    println(left)
    println(product + difference)
    println(alnumBelowBar)
    println(barBelowCons)
    stackTop match
      case Right((f, c)) => println(s"${f("")} $c")
      case Left(n) => println(n)
    println(s"$letter $negative ${self eq this} ${new StringBuilder == null}")
    println(prefixed((1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, "x")))
