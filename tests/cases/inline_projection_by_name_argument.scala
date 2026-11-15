// expected: by hand
// A class's by-name parameter where an inline method's expansion reduces a projection of the class's creation: the
// by-name argument runs only where the parameter is read, as outside an inline method. scalac 3.8.4 runs it once
// for every reduced selection (its reduction keeps each argument that is not elideable for its effect, a by-name
// one too); its line stands beside the call. The expected lines are written by hand: teq's, what the program
// means, and not scalac's for the first call.
object Test:
  val log = new StringBuilder
  def note(k: Int): Int = { log.append(k); k }
  def shown(r: Any): Unit = { println(log.toString + " " + r); log.clear() }

  class Late(val a: Int, b: => Int) { def get: Int = b }
  inline def first(inline l: Late): Int = l.a + new Late(note(3), note(4)).a
  inline def read(inline l: Late): Int = l.get
  def plain(l: Late): Int = l.a + l.a

  def main(args: Array[String]): Unit =
    shown(first(new Late(note(1), note(2)))) // scalac: "1234 4": both by-name arguments run
    shown(plain(new Late(note(1), note(2))) + new Late(note(3), note(4)).a) // scalac: "13 5", as teq: no inline method
    shown(read(new Late(note(1), note(2)))) // scalac: "12 2", as teq: the parameter is read
