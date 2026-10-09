package vcfb

import vcf.*

object Use:
  def main(args: Array[String]): Unit =
    val d = new D(1.5)
    println(d.twice == 3.0)
    println(d.hashCode == new D(1.5).hashCode)
    println(d.equals(new D(1.5)))
    println(d == new D(2.5))
    val c = CV(3)
    println(c)
    println(c.hashCode == CV(3).hashCode)
    println(c.equals(CV(3)))
    println(c.productPrefix + c.productArity + c.productElement(0) + c.productElementName(0))
    println(c.canEqual(c))
    println(new S("s").hashCode == new S("s").hashCode)
    println(new S("s").equals(new S("s")))
