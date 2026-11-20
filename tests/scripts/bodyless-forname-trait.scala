// Initializing an interface initializes none of its superinterfaces (JLS 12.4.1): Class.forName of a trait over a
// body-less Java interface with a static initializer and a default (Meter, and Gauge over it) runs, with one
// argument and with three. Creating a class that implements the trait (Panel) initializes Meter, a superinterface that
// declares a default (JVMS 5.5): refused there.
trait Display extends jlib.Meter
trait Screen extends jlib.Gauge
class Panel extends Screen:
  def level(): Int = 3

@main def run(): Unit =
  println(Class.forName("Display").getName)
  println(Class.forName("Screen", true, classOf[Display].getClassLoader).getName)
  println(new Panel().level())
