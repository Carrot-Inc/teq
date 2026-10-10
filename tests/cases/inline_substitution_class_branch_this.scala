// A class an inline body makes is copied whole at each expansion: a branch of its method is no part
// the walk leaves for a demand, which would copy it where `this` is the expansion's receiver (the
// class's `prod` read on `S`). scalac prints the lines of the .expected file.
trait Der:
  def join(x: Int): String
trait SMD:
  def tag: String = "smd"
  inline def derived(x: Int): String =
    val d: Der = new Der:
      def join(x: Int): String = if x > 100 then "big" + prod(x) else prod(x)
      private def prod(x: Int): String = "p" + x + tag
    d.join(x)
object S extends SMD
@main def run(): Unit =
  println(S.derived(3))
  println(S.derived(300))
