// A `return` in a local lazy val's initialiser leaves the enclosing def, not the initialiser (dotty's NonLocalReturns
// takes a return under a lazy val as non-local): the def's code after the read does not run, in a top-level def, a
// class's method and a value class's result.
case class M(x: Int) extends AnyVal
def a(n: Int): Int =
  lazy val v: Int = return n
  v + 1
def b(n: Int): Int =
  lazy val v: Int = { if n > 1 then return n; 0 }
  v + v + 1
class K:
  def c(n: Int): String =
    lazy val v: Int = { if n > 1 then return "k" + n; 0 }
    "v" + (v + 1)
def d(n: Int): M =
  lazy val v: Int = { if n > 1 then return M(n); 0 }
  M(v + 1)
@main def run(): Unit =
  println(s"a ${a(2)} ${a(0)}")
  println(s"b ${b(2)} ${b(0)}")
  println(s"c ${K().c(2)} ${K().c(0)}")
  println(s"d ${d(2).x} ${d(0).x}")
