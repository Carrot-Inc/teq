package pmdb

import pmda.*

object UsePolyMemberDep:
  val p: Parent { def run[A](a: A): a.type } = new Parent { def run[A](a: A): a.type = a }
  val kept: Parent { def run[A](a: A): a.type } = PolyMemberDep.keep(p)
  def main(args: Array[String]): Unit = println(kept.run[String]("dep"))
