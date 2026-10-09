package pgb

import pga.*
import pga.inner.given

object Use:
  def main(args: Array[String]): Unit =
    println(summon[TC].name)
    println(pga.inner.listTC(using pga.inner.tc).name)
