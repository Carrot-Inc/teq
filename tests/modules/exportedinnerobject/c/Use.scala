package eioc

object Desk extends eiob.Board

import Desk.*

object Use:
  def main(args: Array[String]): Unit =
    shade.seen += 1
    shade.seen += shade.value
    println((shade.value, shade.seen))
