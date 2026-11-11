package rts

import scala.quoted.*

class Pc(val n: Int)
object Pc:
  def apply(n: Int): Pc = new Pc(n)
  def apply(using q: Quotes)(t: q.reflect.Term): Pc = new Pc(2)
