package oia

// An object whose initialiser runs code, one with a constant val alone, one with a statement,
// one whose parent's constructor runs code: a downstream loading each runs what the whole
// program runs.
object Loud:
  val unused = { print("init "); 1 }
  def f = 0

object Quiet:
  val k = 5
  def g = k + 1

object Stmt:
  println("stmt ran")
  def h = 2

class Base(val tag: String):
  print("base(" + tag + ") ")

object WithParent extends Base("p"):
  def i = 3
