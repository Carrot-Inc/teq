// A case's guard that is a match with its cases on the lines below, the case's arrow on a line of its own after
// them. scalac prints ab.
def t(x: Int, y: Int): String = x match
  case 1 if y match
      case 2 => true
      case _ => false
    => "a"
  case _ => "b"
@main def main(): Unit = println(t(1, 2) + t(1, 3))
