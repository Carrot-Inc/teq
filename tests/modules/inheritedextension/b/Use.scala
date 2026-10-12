package ieb

import iea.*

class Clas extends Base:
  def show = Ok.page("x")
  def chained = Ok.twice.page("chained")
  class Inner:
    def fromInner = Ok.twice.twice.page("inner")

@main def main() =
  val c = new Clas
  println(c.show)
  println(c.chained)
  println(new c.Inner().fromInner)
