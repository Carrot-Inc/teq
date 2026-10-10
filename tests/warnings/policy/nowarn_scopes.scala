import scala.annotation.nowarn
object O { val x = 1; val y = 2 }
@deprecated("old", "1") def old(): Int = 1

@nowarn("cat=deprecation")
class Quiet:
  def a = old()
  def b = { import O.x; 2 }

class Loud:
  @nowarn("id=E198") def a = { import O.x; old() }
  def b = (old(): @nowarn)
  def c = (old(): @nowarn("msg=not this"))

@main def run(): Unit = println(Quiet().a + Loud().b)
