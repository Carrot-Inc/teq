// A case whose body is empty, the next case on the line after its arrow (no newline token follows
// `=>`, which ends no statement), as dotc's block of no statements: lila's `case '0' =>`.
@main def run(): Unit =
  var sum = 0
  List(1, 0, 2).foreach: c =>
    c match
      case 1 => sum += 1
      case 0 =>
      case x => sum += x
  println(sum)
  def f(i: Int) = i match
    case 0 =>
    case _ => println("other")
  f(0)
  f(1)
  val g: Int => Unit = {
    case 1 =>
    case n => println(s"n=$n")
  }
  g(1)
  g(5)
