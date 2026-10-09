package lrb

import lra.*

object Use:
  def main(args: Array[String]): Unit =
    println(DeriveEnc.derived[Pet].show)
    println(DeriveEnc.derived[Dog].show)
