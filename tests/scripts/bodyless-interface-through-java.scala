// The same through a Java class whose own constructor is empty (Dial implements Gauge).
class Panel extends jlib.Dial:
  def level(): Int = 3

@main def run(): Unit = println(new Panel().level())
