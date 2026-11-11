// Default getters create real companions; their non-case classes still receive
// NamerOps.addConstructorProxies' constructor applies.
class Top(val n: Int = 5)
trait Owner:
  val base: Int = 11
  class Entry(val n: Int = base)(val read: Unit => Int = _ => n)
class Concrete extends Owner

def localValue(): Int =
  class Local(val n: Int = 23)
  Local().n

@main def run(): Unit =
  println(Top().n)
  println(Top(n = 7).n)
  val owner = new Concrete
  println(owner.Entry()().read(()))
  println(owner.Entry(13)().read(()))
  println(localValue())
