// A function literal of Gauge: its class initializes Meter.
@main def run(): Unit =
  val g: jlib.Gauge = () => 3
  println(g.level())
