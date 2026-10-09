package libb
import liba.A
class B extends A:
  def twiceHello: Int = hello * 2
  def fresh: A = A.make
  def own: Int = twiceHello + helper
  private def helper: Int = 3
