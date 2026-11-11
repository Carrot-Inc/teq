import scala.language.dynamics

class Plain extends Dynamic:
  def applyDynamic(name: String)(): Target = Target(name)

class Using extends Dynamic:
  def applyDynamic(name: String)()(using DummyImplicit): Target = Target(name)

class Selected extends Dynamic:
  def selectDynamic(name: String)(using DummyImplicit): Target = Target(name)

class Target(from: String) extends Dynamic:
  def applyDynamic(name: String)(x: Int): String = s"$from.$name($x)"

@main def run(): Unit =
  println(Plain().go().apply(1))
  println(Using().go().apply(2))
  println(Selected().go.apply(3))
