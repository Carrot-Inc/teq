// `typedAppliedTypeTree` reports `wrongNumberOfTypeArgs` (E023, Typer.scala
// 2733-2735) before it types the arguments, and types only the first `tparams.length`
// of them (`args.take`): an argument's later error (`Not found: type Undef`,
// a kind error `List[Int] does not take type parameters`) lies inside the
// E023's span, which `UniqueMessagePositions` logged, so scalac 3.8.4 hides it
// (it shows with `-Yshow-suppressed-errors`), and a dropped argument is never
// typed. scalac reports the three E023 alone. scalac's columns are 0-based.
trait Validate[T, P] { type R }
object V:
  type Aux[T, P, R0] = Validate[T, P] { type R = R0 }
  type Few[T] = Aux[T, Undef]
  type Many = Aux[Int, String, Int, Undef]
  type Kind[T] = Aux[List[Int][Int], Int]
// expect: wrong_number_of_type_args_hides_its_arguments.scala:11:17: error: Not enough type arguments for V.Aux[T, P, R0]
// expect: wrong_number_of_type_args_hides_its_arguments.scala:12:15: error: Too many type arguments for V.Aux[T, P, R0]
// expect: wrong_number_of_type_args_hides_its_arguments.scala:13:18: error: Not enough type arguments for V.Aux[T, P, R0]
// absent: Undef not found
// absent: does not take type
// expect: 3 errors found
