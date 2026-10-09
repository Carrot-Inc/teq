package ppa

object A:
  private[ppa] def f: Int = 42
  def g: Int = 1
  private[ppa] class C:
    def f: Int = 42
  private[ppa] object O:
    def o: Int = 5
  protected[ppa] class D:
    def d: Int = 6

class K:
  private[ppa] def h(x: Int): Int = x + 1
  private[ppa] val limit: Int = 3

private class Hidden:
  def v: Int = 7
