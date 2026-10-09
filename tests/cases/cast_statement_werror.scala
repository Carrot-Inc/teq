// teq: --werror
// A cast is no pure expression in statement position (dotty's `TreeInfo.exprPurity` takes the
// `TypeApply` of `asInstanceOf` for an effect), so `--werror` compiles a discarded one, as
// scalac's `-Werror` does, whatever its erasure makes of it.
@main def run(): Unit =
  val x: Any = "x"
  x.asInstanceOf[String]
  x.asInstanceOf[Any]
  null.asInstanceOf[Int]
  x.asInstanceOf[Unit]
  println("ok")
