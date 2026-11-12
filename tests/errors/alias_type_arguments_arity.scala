trait Validate[T, P] { type R }
object V:
  type Aux[T, P, R0] = Validate[T, P] { type R = R0 }
  type Few[T, P] = Aux[T]
  type Two[T, P] = Aux[T, P]
  type Many[T, P] = Aux[T, P, Int, String]
  def few(x: Few[Nothing, Int]): Validate[String, Int] = x
  def many(x: Many[String, Int]): Validate[String, Int] = x
  def manyMember(x: Many[String, Int]): x.R = 1
// `typedAppliedTypeTree`'s arity check of a constructor that is no class (Typer.scala 2733-2739, `wrongNumberOfTypeArgs`,
// E023): an alias applied to fewer or more arguments than its parameters. scalac 3.8.4 reports the three at 4:19, 5:19
// and 6:20 with these messages; too many arguments apply the first ones (`Many[String, Int]`'s `R` is `Int`), too few
// make an application scalac's comparer has above and below every type (`few` is accepted). teq reported none before
// this check, applying the alias to what it was given.
// expect: alias_type_arguments_arity.scala:4:20: error: Not enough type arguments for V.Aux[T, P, R0]
// expect: alias_type_arguments_arity.scala:5:20: error: Not enough type arguments for V.Aux[T, P, R0]
// expect: alias_type_arguments_arity.scala:6:21: error: Too many type arguments for V.Aux[T, P, R0]
// expect: 3 errors found
