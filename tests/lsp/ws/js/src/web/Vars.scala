package web

trait Slot:
  var level: Int
  def raise(): Unit = level = level + 1

object Vars:
  def fill(s: Slot): Unit =
    s.level = 3
    s.level += 1
