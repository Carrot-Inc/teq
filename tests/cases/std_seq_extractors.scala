// The +: and :+ extractors of scala-library on a sequence, splitting off its head and its last
// element.
@main def main(): Unit =
  Vector(1, 2, 3) match
    case h +: rest => println(s"$h $rest")
    case _ => println("empty")
  Vector(1, 2, 3) match
    case init :+ last => println(s"$init $last")
    case _ => println("empty")
