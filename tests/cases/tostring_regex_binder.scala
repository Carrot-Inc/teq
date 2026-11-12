// teq: --wtostring-interpolated --werror
//> using options -Wtostring-interpolated -Werror
// The binders of a `Regex` pattern are `String`s: scala-library declares `Regex.unapplySeq` as returning
// `Option[List[String | Null]]`, and dotty gives a binder the simplified type of its pattern (`Typer.typedBind` takes
// the typed body's type, which `Typer.simplify` overwrote with `tpe.simplified`: `TypeOps.simplify`'s `OrType` arm
// drops `Null` under `String`). So interpolating them is no use of `toString`: scalac 3.8.4 compiles this under
// `-Wtostring-interpolated -Werror` and prints `12 to 34`.
@main def run(): Unit =
  val re = "(\\d+)-(\\d+)".r
  "12-34" match
    case re(a, b) => println(s"$a to $b")
    case _        => println("no")
