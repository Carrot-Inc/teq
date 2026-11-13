// The assignment operators through inline getters: arithmetic ones, a right-associative one (`::=`, whose
// value is the operand's `::` of the read), `:+=`, and one over a Boolean.
package meters

class Meter:
  private var held = 10
  private var names = List("a")
  private var open = false
  inline def level: Int = held
  def level_=(v: Int): Unit = held = v
  inline def all: List[String] = names
  def all_=(v: List[String]): Unit = names = v
  inline def ready: Boolean = open
  def ready_=(v: Boolean): Unit = open = v

@main def run(): Unit =
  val m = Meter()
  m.level -= 3
  m.level *= 2
  m.all ::= "b"
  m.all :+= "c"
  m.ready ||= true
  println(s"${m.level} ${m.all.mkString} ${m.ready}")
