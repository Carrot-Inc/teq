package lib.other

object Main:
  def main(args: Array[String]): Unit =
    given Int = 11
    val i = new use.Impl
    println(List(i.g, lib.inner.K(1).copy(x = 2).x, lib.inner.E.A.m, lib.inner.O.m, new use.C().m, new lib.inner.D().m()).mkString(" "))
