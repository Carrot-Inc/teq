// `a op (x, y)` passes two arguments; a lone non-repeated parameter receives them as a tuple.

final class Acc(val items: List[String]):
  infix def add(a: Int, b: Int): Acc = Acc(items :+ ("add:" + (a + b).toString))
  infix def pair(p: (Int, Int)): Acc = Acc(items :+ ("pair:" + p.toString))
  infix def any(p: Any): Acc = Acc(items :+ ("any:" + p.toString))
  infix def gen[T](p: T): Acc = Acc(items :+ ("gen:" + p.toString))
  infix def rep(ps: Int*): Acc = Acc(items :+ ("rep:" + ps.length.toString))
  infix def pairs(ps: (Int, Int)*): Acc = Acc(items :+ ("pairs:" + ps.toList.toString))
  infix def dflt(a: Int, b: Int = 10): Acc = Acc(items :+ ("dflt:" + (a + b).toString))
  infix def lzy(p: => (Int, String)): Acc = Acc(items :+ ("lzy:" + p.toString))
  def <+>(a: Int, b: String): Acc = Acc(items :+ ("<+>:" + a.toString + b))
  def +:(p: (Int, String)): Acc = Acc(("+::" + p.toString) :: items)
  def ++:(a: Int, b: String): Acc = Acc(("++::" + a.toString + b) :: items)

extension (acc: Acc)
  def <*>(a: Int, b: Int): Acc = Acc(acc.items :+ ("<*>:" + (a * b).toString))
  def <~>(p: (Int, Int)): Acc = Acc(acc.items :+ ("<~>:" + p.toString))
  infix def put(a: Int): Acc = Acc(acc.items :+ ("put1:" + a.toString))
  infix def put(a: Int, b: Int): Acc = Acc(acc.items :+ ("put2:" + (a + b).toString))

object cls:
  def :=(classes: (String | (String, Boolean))*): String =
    var out = ""
    classes.foreach: c =>
      val name = c match
        case s: String => s
        case (s: String, on: Boolean) => if on then s else ""
      if name.nonEmpty then out = (if out.isEmpty then name else out + " " + name)
    out

def takesPair(p: (Int, Int)): Int = p._1 + p._2
def takesAny(p: Any): String = p.toString
def takesLazy(p: => (Int, Int)): Int = p._1 * p._2
def generic[T](p: T): String = "<" + p.toString + ">"

final case class Wrap(p: (Int, String))

@main def main(): Unit =
  val active = true

  println(cls := ("flex", "bold" -> active, "hidden" -> !active, if active then "on" else "off"))
  println(cls := ("only"))
  println(cls := "plain")
  println(cls := ("pair" -> active))
  println(cls := (("pair2", active)))
  println(cls := (("p", true), ("q", false), "r"))

  val a = Acc(Nil) add (1, 2) pair (3, 4) any (5, 6) gen (7, 8) rep (1, 2, 3) dflt (1, 2) dflt (1)
  println(a.items)
  val b = Acc(Nil) pairs ((1, 2)) pairs ((1, 2), (3, 4)) lzy (1, "one") any ((1, 2))
  println(b.items)
  val c = Acc(Nil) <+> (1, "a") <*> (3, 4) <~> (5, 6) put (1) put 4
  println(c.items)
  println(c.put(1, 2).put(3).items.length)

  val d = (1, "x") +: Acc(Nil)
  println(d.items)
  val e = (2, "y") ++: (3, "z") +: Acc(Nil)
  println(e.items)

  println(1 -> (2, 3))
  println((1, 2) -> (3, 4))
  println((1, 2) :: List((3, 4)))
  println((1, 2) :: (3, 4) :: Nil)
  println(List((1, 2)) :+ (3, 4))
  println(List(1) :+ 2)
  var xs = List((1, "a"))
  xs :+= (2, "b")
  xs ::= (0, "z")
  println(xs)
  println("s" + (1, 2))
  println((1, 2) == (1, 2))
  println(Some((1, 2)) == Some(1 -> 2))

  println(takesPair(1, 2))
  println(takesPair((3, 4)))
  println(takesAny(1, 2))
  println(takesLazy(5, 6))
  println(generic(1, "two", 3))
  val f: ((Int, Int)) => Int = p => p._1 * p._2
  println(f(3, 4))
  println(Wrap(1, "w"))
  println(Wrap(1, "w").copy(2, "v"))
  println(Some(1, 2))
  println(Option(1, 2).map(_._2))
