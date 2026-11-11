import a.*
@main def run(): Unit =
  println(m1.use1() + m2.use2() + m3.use3() + m4.use4())
  println(g())
  println(Trace.lines.reverse.mkString(","))
