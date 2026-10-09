package anb

import ana.ApproxNarrow

object UseApproxNarrow:
  def main(args: Array[String]): Unit =
    val g: String ?=> List[Option[Nothing]] => Int = ApproxNarrow.widen[Int](xs => summon[String].length + xs.size)
    println(g(using "abc")(Nil))
