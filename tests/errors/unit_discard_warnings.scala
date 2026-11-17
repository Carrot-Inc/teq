// teq: --werror
// The discard's warning and the pure statement's ask one test of purity: a value is told where the typing
// drops it (a branch, a case, a handler), at its point (the bracket of a creation, the name of a selection);
// an expansion with an effect, an object with an initialiser, `this` and an ascription to `Unit` are not told.
object Still:
  def kept: Int = 3

object Loud:
  println("loud")
  val kept: Int = 4

class Shelf(val side: Int):
  def me: Unit = if side > 0 then this else ()

transparent inline def pureCall: Int = 4
transparent inline def loudCall: Int = { println("effect"); 1 }

def flag: Boolean = true
def branch: Unit = if flag then 1 else "two"
def cases(k: Int): Unit = k match { case 0 => 2.5; case _ => 'c' }
def handler: Unit = try 3 catch { case _: RuntimeException => null }
def expansion: Unit = if flag then pureCall else loudCall
def objects: Unit = if flag then Still else Loud
def selection(s: Shelf): Unit = if flag then s.side else new Shelf(5).side
def ascribed: Unit = if flag then (6: Unit) else ((7: Unit): Unit)
def statements(s: Shelf): Unit = { 8; s.side; Still; Loud; (9: Unit); () }
def local(s: Shelf): Unit = { val u: Unit = pureCall; u }
def created(s: Shelf): Unit = if flag then new Shelf(s.side) else (s, 11)
def blockCondition: Unit = if { println("hi"); flag } then 12 else 13
def nested(a: Any): Unit = a match { case s: Shelf => (s: Any) match { case t: Shelf => () => (s, t) } }
