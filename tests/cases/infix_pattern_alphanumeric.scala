// An extractor named by an identifier between two patterns (`case a withKey b`), not only by an operator, and the
// operators of an infix pattern grouped by their precedence and associativity as terms are (dotty's
// `infixPattern` over `infixOps`): `a ~ b ~ c` is `(a ~ b) ~ c`, `a :: b :: c` is `a :: (b :: c)`, and `::`
// binds tighter than an alphanumeric extractor.
// teq: --wconf msg=is.not.declared.infix:s
object withKey:
  def unapply(p: (String, Int)): Some[(String, Int)] = Some(p)
object ~ :
  def unapply(p: Pair): Some[(Any, Any)] = Some((p.l, p.r))
case class Pair(l: Any, r: Any)
object and:
  def unapply(xs: List[Int]): Option[(List[Int], List[Int])] = Some(xs.splitAt(1))

@main def main(): Unit =
  ("a", 1) match
    case name withKey key => println(s"$name $key")
  val label: ((String, Int)) => String = { case n withKey k => n * k }
  println(label(("x", 3)))
  Pair(Pair(1, 2), 3) match
    case a ~ b ~ c => println(s"left: $a $b $c")
    case _ => println("no")
  Pair(1, Pair(2, 3)) match
    case a ~ (b ~ c) => println(s"right: $a $b $c")
    case _ => println("no")
  List(1, 2, 3) match
    case x :: rest and more => println(s"$x $rest $more")
    case _ => println("none")
