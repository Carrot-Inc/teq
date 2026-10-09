def traced(using t: Tracer.instance.Type): String = t.toString

@main def run(): Unit =
  println(traced)
  val t: Tracer.instance.Type = Tracer.explicit
  println(t)
