// A match type nested in a case of a bounded match alias is typed against the alias's bound (dotty's
// `typedTypeCase` passing its `pt` to `typedMatchTypeTree`): where a body reduces further the nested match is
// bounded by it, so an application stuck on an abstract scrutinee still conforms to the bound.
type Pick[T <: Tuple, N <: Int] <: Tuple = N match
  case 0 => EmptyTuple
  case _ => T match
    case EmptyTuple => EmptyTuple
    case x *: xs => x *: Pick[xs, 0]

def pick(t: Tuple): Tuple = (t.asInstanceOf[Pick[Tuple, 1]]: Tuple)

@main def main(): Unit =
  val p: Pick[(Int, String), 1] = (1 *: EmptyTuple)
  println(p)
  println(pick((2, "b")))
