package dtfb

import dtfa.*

object UseTypes:
  def inferred(c: Ctx): c.T = Types.inferred(c)
  def explicit(c: Ctx): c.T = Types.explicit(c)
  def alias(f: Types.F, c: Ctx): c.T = f(c)
  def inherited(d: Derived, c: Ctx): c.T = d.inherited(c)
  def main(args: Array[String]): Unit =
    val c = new Ctx { type T = String; val t = "types" }
    val s: String = inferred(c)
    println(s + " " + explicit(c) + " " + alias(Types.explicit, c) + " " + inherited(Derived(), c))
