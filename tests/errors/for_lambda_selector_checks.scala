// teq: --source 3.8 --werror
// A generator's case lambda under N parameters takes its selector's check, `IrrefutablePatDef` where the generator's
// was `None` (`typedFunctionValue`, Typer.scala 2024-2025): a pattern definition's message at the pattern; an
// `@unchecked` selector silences the type tests made on it alone, a component's test warns (the review of 1888ac4df).
class Z(xs: List[(Int, Option[Int])]):
  def withFilter(p: ((Int, Option[Int])) => Boolean): Z2 = Z2(xs.filter(p))
class Z2(xs: List[(Int, Option[Int])]):
  def foreach(f: (Int, Option[Int]) => Unit): Unit = xs.foreach((a, b) => f(a, b))
@main def m =
  for case (a, Some(b)) <- Z(List((1, Some(2)), (3, None))) do println(a + b)
  val as: List[Any] = List(List(1))
  for case x: List[Int] <- as do println(x)
  for case (y: List[Int], z) <- List[(Any, Int)]((List(1), 2)) do println(y)
  for case w @ ((_: List[Int]) | (_: Set[Int])) <- as do println(w)
// expect: 10:12: warning: pattern's type Some[Int] is more specialized than the right hand side expression's type Option[Int]
// expect: adding `.runtimeChecked` after the expression,
// expect: 13:13: warning: the type test for List[Int] cannot be checked at runtime because its type arguments can't be determined from Any
// absent: for_lambda_selector_checks.scala:12:
// absent: for_lambda_selector_checks.scala:14:
