val int4: Codec[Int] = Codec("int4")
val text: Codec[String] = Codec("text")

@main def run(): Unit =
  val f: Frag[Int] = Mac.mk(int4)
  println(f.show)
  val g: Frag[Int *: String *: EmptyTuple] = Mac.mk(int4, text)
  println(g.show)
  val h: Frag[(String, Int, Int)] = Mac.mk(text, int4, int4)
  println(h.show)
