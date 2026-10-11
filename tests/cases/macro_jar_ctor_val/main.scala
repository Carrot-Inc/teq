import ctorinitlib.*

@main def run(): Unit =
  println(Probe.recorded)
  println(Recorder("x", "y", "z").log)
  println(Counter().seen.mkString(","))
