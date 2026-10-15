// A colon lambda's body on the arrow's line inside a guard is no colon lambda (`isArrowIndent`): refused at the
// `:`, as scalac refuses it.
// expect: 6:29: error: expected '=>', found ':'
@main def run(): Unit =
  println(1 match
    case n if List(1).exists: x => x > 0

    && true => n
    case _ => 0)
