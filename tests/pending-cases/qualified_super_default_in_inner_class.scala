// A call through the enclosing class's `super` from an
// inner class (`Q.super.pm`), its default left out (`g`)
// and given (`g4`): scalac 3.8.4 prints the .expected, P's
// `pm` with P's default, past Q's override. teq is to print
// it too, in every mode (JVM, JavaScript, interpreter).
class P:
  def pm(y: Int = 3): Int = y

class Q extends P:
  override def pm(y: Int = 7): Int = y * 100
  class Inner:
    def g: Int = Q.super.pm()
    def g4: Int = Q.super.pm(4)

@main def run(): Unit =
  val q = Q()
  val i = q.Inner()
  println(i.g)
  println(i.g4)
