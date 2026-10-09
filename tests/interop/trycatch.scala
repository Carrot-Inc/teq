def parse(s: String): Either[String, Int] =
  js.tryCatch[Either[String, Int]](() =>
    if s.isEmpty then js.throwError("empty input")
    else Right(js.cast[Int](js.call(js.global("JSON"), "parse", s)))
  )(e => Left("failed: " + e.toString.take(12)))

@main def main(): Unit =
  println(parse("42"))
  println(parse(""))
  println(parse("{oops"))
  var log = List.empty[String]
  val r = js.tryCatch(() => js.tryFinally(() => { js.throwError("boom"); 1 })(() => log = "finalized" :: log))(_ => -1)
  println(r)
  println(log)
