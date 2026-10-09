package ilb

import ila.*

object Use:
  def main(args: Array[String]): Unit =
    println(Macros.plain(1))
    println(new Tuner().run(3))
