//> using platform js
// A number expected to be a union with one numeric member converts to that member, as it does
// against the member alone: an Int becomes the Double of `Double | String`.
class Options(val maxSizeMB: Double | Unit = (), val label: Double | String = "x"):
  override def toString: String =
    val size = maxSizeMB match
      case d: Double => d.toString
      case _ => "none"
    s"Options($size, $label)"

def kind(x: Long | String): String = x match
  case l: Long => "long " + (l + 1L)
  case s: String => "string " + s

def kindD(x: Double | String): String = x match
  case d: Double => "double " + (d / 2)
  case s: String => "string " + s

def half(x: Double | Unit): Double = x match
  case d: Double => d / 2
  case _ => -1

def ratio: Double | String = 7

@main def main(): Unit =
  val i: Int = 3
  println(Options(maxSizeMB = i))
  println(Options(maxSizeMB = 3))
  println(Options(label = 4))
  println(kind(i))
  println(kind(8))
  println(kind("s"))
  println(kindD(i))
  println(kindD(7))
  println(kindD(2L))
  println(kindD('a'))
  println(half(i))
  println(half(5))
  println(ratio)
  val u: Double | Unit = i
  val v: Double | String = 9
  println(half(u) + half(v match { case d: Double => d; case _ => 0 }))
  val opt: Option[Double | String] = Some(i)
  println(opt.map(kindD))
  val xs: List[Double | String] = List(1, "a", 2.5)
  println(xs.map(kindD))
  val ys: List[Long | String] = List(1, "a", 2L)
  println(ys.map(kind))
