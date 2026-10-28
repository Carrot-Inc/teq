trait Kind:
  type Up
  type Count[X <: Tuple] <: Int = X match
    case EmptyTuple => 0
    case Up *: t => Count[t]
    case _ *: t => Count[t]
object K extends Kind:
  type Up = Int
