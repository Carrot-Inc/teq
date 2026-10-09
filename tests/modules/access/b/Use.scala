package acb

import aca.*

class Derived extends Base:
  override protected def hook: String = "derived"
  def exposed: String = hook

object Use:
  def main(args: Array[String]): Unit =
    val d = new Derived
    println(d.describe + " " + d.exposed + " " + Friends.peek(d))
