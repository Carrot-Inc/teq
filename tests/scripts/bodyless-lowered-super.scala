// A super call into a Java interface's default in lowered code (the method's second call) is placed at the super
// call, as the walker places it.
class Lamp(loud: Boolean) extends jlib.Quiet:
  override def hum(): String = if loud then super.hum() + "!" else "quiet"

@main def run(): Unit =
  println(new Lamp(false).hum())
  println(new Lamp(true).hum())
