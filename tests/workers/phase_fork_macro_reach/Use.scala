import Lit.*

object Use:
  def a: Model.Code = code"ABC"
  def b: Model.Code = code"XYZ"

@main def run(): Unit =
  println(summon[Show[Model.Code]].show(Use.a))
  println(summon[Show[Model.Code]].show(Use.b))
  println(summon[Show[Model.Name]].show(Model.Name("n")))
  println(Use2.c)
