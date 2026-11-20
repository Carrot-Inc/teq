// Gauge's parent Meter has an initializer and a default: the creation initializes it (JVMS 5.5).
class Panel extends jlib.Gauge:
  def level(): Int = 3

@main def run(): Unit = println(new Panel().level())
