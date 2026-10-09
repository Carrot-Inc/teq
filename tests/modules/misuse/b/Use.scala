package mub

import mua.*

object Use:
  def main(args: Array[String]): Unit =
    val a = new Account("me")
    a.deposit("ten")
    println(a.secret)
