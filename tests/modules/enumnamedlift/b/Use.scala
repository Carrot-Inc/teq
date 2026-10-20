package enluse

import enl.*

object Use:
  def f = Group(connective = Marks.mark("c", Conn.And), items = Marks.mark("i", List(1)))
  def g = Group(connective = Conn.Or, items = Nil)
  def h(c: Conn) = Group(connective = c, items = List(2))
  def k = Key("a", 1)
  def l = Key(disabled = true, name = "b", keyType = 2)
  def m = Run(mode = Marks.mark("m", Mode.Fast), items = Marks.mark("i", List(3)))

@main def main(): Unit =
  println(Use.f); println(Marks.log)
  println(Use.g)
  println(Use.h(Conn.And))
  println(Use.k)
  println(Use.l)
  println(Use.m); println(Marks.log)
