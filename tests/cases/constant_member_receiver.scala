// A constant member stands for its value, after its receiver where that is no stable path.
class C:
  println("  made")
  final val yes = true
  final val limit = 3
  final val name = "c"

object Holder:
  println("  init Holder")
  final val on = true

def getC(): C =
  println("  getC")
  new C

def none(): C =
  println("  none")
  null

object O:
  println("  init O")
  final val flag = getC().yes
  final val again = O.flag

inline def chosen: String = inline if O.flag then "chosen" else "not chosen"

@main def main(): Unit =
  println("behind a def")
  println(getC().yes)
  println("behind new")
  println(new C().limit)
  println("behind a var")
  var v = getC()
  println(v.name)
  println("behind a val")
  val c = getC()
  println(c.yes)
  println("behind a lazy val")
  lazy val lz = getC()
  println(lz.yes)
  println("behind an object")
  println(Holder.on)
  println("in a condition")
  println(if getC().yes then "then" else "else")
  println("in an argument")
  println(List(getC().limit, new C().limit).sum)
  println("a constant defined through a receiver")
  println(O.flag)
  println(O.again)
  println(chosen)
  println("a null receiver")
  try println(none().yes)
  catch case _: Throwable => println("  caught")
