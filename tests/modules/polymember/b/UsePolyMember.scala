package pmb

import pma.*

object UsePolyMember:
  val p: Parent { def run[A](a: A): A } = new Parent { def run[A](a: A): A = a }
  val kept: Parent { def run[A](a: A): A } = PolyMember.keep(p)
  def main(args: Array[String]): Unit = println(kept.run[String]("ok"))
