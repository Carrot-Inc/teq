/*
rules = ["file:../../support/DumpLayout.scala"]
*/
// A pattern in parentheses wherever the grammar takes one: the parentheses are no node, and the construct that holds
// them spans them, an infix pattern's argument clause its parentheses, as scalameta 4.17's.
object ParenPatterns:
  val marker = 0 // assert: DumpLayout.layout
  def values(x: Any): Int = x match
    case (1) => 1
    case Some((2)) => 2
    case (3) | (4) => 3
    case y @ (Some(_)) => 4
    case (_: String) => 5
    case ((a: Int), (b: Int)) => a + b
    case ((6)) => 6
    case _ => 0
  def lists(xs: List[Int]): Int = xs match
    case (h) :: (t) :: Nil => h
    case (h) :: Nil => h
    case h :: (t) => h
    case _ => 0
  val (p) = 1
  val (q, (r)) = (1, 2)
  def generators = for ((x) <- List(1); ((y), z) <- List((1, 2))) yield x + y + z
  def more(x: Any): Int = x match
    case (-1) => 1
    case (s: String) :: Nil => 2
    case List((a), b) => 3
    case z @ (_: Int) => 4
    case _ => 0
  val (a: Int) = 1
  object ~~ :
    def unapply(p: (Int, (Int, Int))): Option[(Int, (Int, Int))] = Some(p)
  def layers(xs: List[Int], t: (Int, (Int, Int))): Int = (xs, t) match
    case (h :: ((t)), _) => h
    case ((h) :: ((u)) :: Nil, _) => h
    case (_, a ~~ (b, c)) => a + b + c
    case (_, a ~~ ((b, c))) => a + b + c
