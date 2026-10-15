// A type continued after a trailing `|` (`infixOps`'s `newLineOptWhenFollowing`).
type T = Int |
  String
@main def run(): Unit = { val x: T = 1; println(x) }
