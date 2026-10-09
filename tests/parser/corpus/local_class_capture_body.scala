// A local class that a subclass extends initialises its vals from what it captures.
package localclasscapturebody

def run(z: Int): Int =
  abstract class L:
    val base = z * 2
    def get: Int = base + extra
    def extra: Int
  val l = new L:
    def extra: Int = z
  l.get

@main def main(): Unit =
  println(run(5))
