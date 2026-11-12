// teq: --wtostring-interpolated --werror
//> using options -Wtostring-interpolated -Werror
// A binder over a binder takes the inner one's type (`Typer.typedBind`): over a literal or a stable identifier that
// is the expected type, `String | Null` here, and interpolating it uses `toString`. scalac 3.8.4 gives the two
// warnings, errors under `-Werror`.
val stable: String = "s"

def literal(v: String | Null): String = v match
  case x @ (y @ "s") => s"$x"
  case _ => "other"

def identifier(v: String | Null): String = v match
  case x @ (y @ `stable`) => s"$x"
  case _ => "other"

@main def run(): Unit =
  println(literal("s"))
  println(identifier("s"))
// expect: 9:28: warning: interpolation uses toString
// expect: 13:33: warning: interpolation uses toString
