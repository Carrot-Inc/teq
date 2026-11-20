// Java names typed, a class tested, and a creation whose Java level is proven from its jar alone: the run reads
// nothing of the JDK's runtime image (src/interp/bodyless.rs).
class Tick extends jlib.Counter

@main def run(): Unit =
  val g: jlib.Gauge = null
  println(g == null)
  println(classOf[jlib.Registry].getName)
  println(new Tick().count)
