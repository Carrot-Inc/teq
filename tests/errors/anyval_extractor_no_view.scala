// expect: 13:20: error: None of the overloaded alternatives of method unapply in object E with types
// expect: 1 error found
// Without a view from `Any` to `AnyVal` (scalac under `-Yno-predef`; the lean std, whose
// Predef has `->` as an extension and no value classes taking any value) neither alternative
// applies in either round of the overload resolution (E134).
object E:
  def unapply(x: String): Option[String] = Some("string")
  def unapply(x: AnyVal): Option[String] = Some("val")

@main def run(): Unit =
  val x: Any = 1
  val s =
    x match { case E(s) => s; case _ => "none" }
  println(s)
