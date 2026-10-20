// Plain inline calls in classes of a block (a local class, an anonymous class, a SAM's lambda) wait
// for the inlining phase as every plain call does; their expansions summon a given of the method
// around the class that the calls do not name, which the class captures once the phase expanded
// them, as dotty's `LambdaLift` runs after `Inlining`.
import scala.compiletime.summonInline

trait Show[A]:
  def show(a: A): String

inline def render[A](a: A): String = summonInline[Show[A]].show(a)

def make(prefix: String): List[String] =
  given Show[Int] with
    def show(a: Int): String = prefix + a
  class Local(n: Int):
    def go: String = render(n)
    class Inner:
      def deeper: String = render(n * 10)
  val anon = new Function0[String]:
    def apply(): String = render(7)
  val sam: java.util.function.Supplier[String] = () => render(8)
  val one = Local(1)
  List(Local(42).go, one.Inner().deeper, anon(), sam.get())

@main def run(): Unit =
  make("n=").foreach(println)
