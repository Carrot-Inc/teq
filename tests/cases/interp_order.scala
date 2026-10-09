import scala.collection.mutable.ArrayBuffer

class Counter:
  var n = 0
  def next(): Int =
    n += 1
    n
  override def toString = s"Counter($n)"

@main def main(): Unit =
  val c = Counter()
  println(s"$c ${c.next()} $c ${c.next()} $c")
  println(raw"$c\n ${c.next()} $c")
  println(f"$c ${c.next()}%03d $c")
  val buf = ArrayBuffer(1, 2)
  println(s"$buf ${buf += 3} $buf ${buf.remove(0)} $buf")
  var i = 0
  def inc(): Int =
    i += 1
    i
  println(s"${inc()} $i ${inc()} $i")
  println(s"${inc()}")
  println(s"$i ${i}")
  val sb = new StringBuilder("x")
  println(s"$sb ${sb.append("y")} $sb")
  def sideStr(): String =
    sb.append("!")
    "s"
  println(s"$sb${sideStr()}$sb")
  println(s"${c.next()}${c.next()}${c.next()}")
