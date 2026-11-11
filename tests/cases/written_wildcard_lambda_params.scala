// A lambda parameter without a type takes a written wildcard formal's lower bound (Typer.typedFunctionValue's
// `formalBounds.loBound`): `String` for `? >: String`, `Nothing` for `?`; an untupled formal and a context
// function's formal (`makeContextualFunction`) read it the same way.
@main def run(): Unit =
  val length: Function1[? >: String, Int] = x => x.length
  println(length.asInstanceOf[String => Int]("abc"))
  val constant: Function1[?, Int] = x => 1
  println(constant != null)
  val pair: Function1[? >: (String, Int), Int] = (s, n) => s.length + n
  println(pair.asInstanceOf[((String, Int)) => Int](("ab", 3)))
  val contextual: ContextFunction1[? >: String, Int] = summon[String].length
  def withText(f: String ?=> Int): Int = f(using "abcd")
  println(withText(contextual))
  val cases: Function2[? >: String, ? >: Int, Int] = { case (s, n) => s.length + n }
  println(cases.asInstanceOf[(String, Int) => Int]("abc", 2))
