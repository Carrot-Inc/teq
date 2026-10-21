//> using scala 3.8.4
//> using dep org.scalameta::scalameta:4.17.3
@main def opts(): Unit =
  val o = summon[scala.meta.tokenizers.TokenizerOptions]
  println("groupWhitespace=" + o.groupWhitespace)
