//> using platform js
object Log:
  def t[A](label: String, a: A): A =
    println("  eval " + label)
    a

import Log.t

class Box(var v: Int):
  def add(a: Int, b: Int): Int = v + a + b
  def +:(x: Int): Box =
    println("  +: called with " + x)
    Box(v + x)
  def ::(x: Int): Box =
    println("  :: called with " + x)
    Box(v + x)
  def update(i: Int, x: Int): Unit =
    println("  update(" + i + ", " + x + ")")
    v = x
  def apply(i: Int): Int =
    println("  apply(" + i + ")")
    v

def three(a: Int, b: Int = t("default-b", 10), c: Int = t("default-c", 20)): Int = a + b + c
def dep(a: Int, b: Int = -1)(c: Int = t("default-c-from-a", 0)): Int = a + b + c
def depPrev(a: Int, b: Int = 7): Int = a * b
def prev(a: Int)(b: Int = a * 2): Int = a + b
def byName(a: => Int, b: Int): Int =
  println("  in byName body")
  a + a + b
def varargs(xs: Int*): Int = xs.sum


@main def main(): Unit =
  println("receiver then args")
  println(t("recv", Box(1)).add(t("a", 1), t("b", 2)))
  println("named args out of order")
  println(three(c = t("c", 3), a = t("a", 1)))
  println(three(c = t("c", 3), b = t("b", 2), a = t("a", 1)))
  println("defaults")
  println(three(t("a", 1)))
  println(dep(t("a", 1))())
  println(prev(t("a", 5))())
  println("by-name")
  println(byName(t("byname-a", 1), t("strict-b", 2)))
  println("varargs")
  println(varargs(t("v1", 1), t("v2", 2), t("v3", 3)))
  println("interpolation")
  println(s"${t("i1", 1)} and ${t("i2", 2)}")
  println("right assoc")
  val r = t("left", 5) +: t("right", Box(1))
  println(r.v)
  val r2 = t("left", 5) :: t("right", Box(1))
  println(r2.v)
  val r3 = t("l1", 1) :: t("l2", 2) :: t("rbox", Box(0))
  println(r3.v)
  val l = t("h", 1) :: t("tl", List(2))
  println(l)
  val l2 = t("h", 1) +: t("tl", Vector(2))
  println(l2)
  val l3 = t("pre", List(0)) ::: t("post", List(2))
  println(l3)
  val l4 = t("pre", List(0)) ++: t("post", List(2))
  println(l4)
  println("assignment ops")
  val arr = Array(1, 2, 3)
  var i = 0
  def idx(): Int =
    println("  idx")
    i += 1
    i
  val bx = Box(5)
  def getBox(): Box =
    println("  getBox")
    bx
  getBox().v += t("rhs", 1)
  println(bx.v)
  getBox()(t("i", 1)) = t("rhs", 99)
  println(bx.v)
  println("tuple / list literals")
  val tp = (t("t1", 1), t("t2", 2))
  println(tp)
  println("boolean short-circuit")
  println(t("f", false) && t("never", true))
  println(t("t", true) || t("never", false))
  println(t("t", true) & t("always", false))
  println("if / match scrutinee")
  println(t("x", 1) + t("y", 2) * t("z", 3))
  println("string concat")
  println(t("s1", "a") + t("s2", 1) + t("s3", 2.0))
  println("compare")
  println(t("c1", 1) == t("c2", 1))
  println(t("c1", "a") != t("c2", "b"))
