package tpb

import tpa.*

object Use:
  def one(l: Label): Label = label(prefix + l)
  def two(l: Label): List[Label] = List(label(l), prefix)

@main def run(): Unit =
  println(Use.one(" x "))
  println(Use.two(" y "))
