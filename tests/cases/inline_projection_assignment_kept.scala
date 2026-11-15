// expected: by hand
// An assignment to a `var` parameter of a creation in an inline method's body: the assignment is made to the
// selection, as outside an inline method. scalac 3.8.4 refuses the first two assignments below with E052
// "Reassignment to val v" at their calls (its inliner puts the argument in the selection's place before it looks
// for a variable), and compiles the others. The expected lines are written by hand: teq's, what the program
// means; scalac compiles no file that holds the first two calls.
object Test:
  val log = new StringBuilder
  def note(k: Int): Int = { log.append(k); k }
  def shown(r: Any): Unit = { println(log.toString + " " + r); log.clear() }

  class Slot(var v: Int)
  class Rack(val slot: Slot, val k: Int)
  inline def assigned(inline s: Slot): Int = { s.v = 5; s.v }
  inline def assignedOwn(k: Int): Unit = new Slot(k).v = 5
  inline def through(inline r: Rack): Unit = r.slot.v = 7
  inline def raised(inline s: Slot): Unit = s.v += 1
  inline def kept(s: Slot): Int = { s.v = 5; s.v += 1; s.v }

  def main(args: Array[String]): Unit =
    shown(assigned(new Slot(note(1)))) // scalac: E052 at this call; the `inline` argument is evaluated at each use
    shown({ assignedOwn(note(2)); "own" }) // scalac: E052 at this call
    shown({ through(new Rack(new Slot(note(3)), note(4))); "through" }) // scalac: "34 through": the receiver is reduced, the target stays
    shown({ raised(new Slot(4)); "raised" }) // scalac: " raised"
    shown(kept(new Slot(note(5)))) // scalac: "5 6"
