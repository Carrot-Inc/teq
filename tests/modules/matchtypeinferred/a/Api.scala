package mti

// The result types are inferred: each local alias is avoided, so the pickled result type is the match type
// itself (MATCHtype), whose capturing case is a type lambda.
object Api:
  def head[X](x: X) =
    type E = X match
      case List[t] => t
      case Option[t] => t
    (x match
      case xs: List[?] => xs.head
      case o: Option[?] => o.get
    ).asInstanceOf[E]
