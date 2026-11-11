import scala.compiletime.ops.int.S

type Drop[T <: Tuple, N <: Int] <: Tuple = N match
  case 0 => T
  case S[n1] => T match
    case EmptyTuple => EmptyTuple
    case x *: xs => Drop[xs, n1]

type Count[N <: Int] <: Int = N match
  case 0 => 0
  case S[n] => Count[n]

object Lists:
  type Reverse[Acc <: Tuple, L <: Tuple] <: Tuple = L match
    case EmptyTuple => Acc
    case h *: t => Reverse[h *: Acc, t]

@main def matchAliasDeclaredBound(): Unit =
  val rest: Drop[(Int, String, Boolean), 1] = ("a", true)
  val zero: Count[3] = 0
  val reversed: Lists.Reverse[EmptyTuple, (Int, String)] = ("b", 1)
  println(rest)
  println(zero)
  println(reversed)
