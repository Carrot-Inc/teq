//> using platform js
// A sensor packet: readings as bytes and shorts, calibrated in single precision.
final case class Packet(id: Byte, raw: Short, gain: Float)

def checksum(bytes: Array[Byte]): Byte =
  var acc = 0
  for b <- bytes do acc = acc + b
  acc.toByte

def calibrate(p: Packet): Float = p.raw * p.gain

def describe(x: Any): String = x match
  case b: Byte => s"byte $b"
  case s: Short => s"short $s"
  case f: Float => s"float $f"
  case i: Int => s"int $i"
  case other => s"other $other"

def sign(f: Float): String = f match
  case 0f => "zero"
  case x if x < 0 => "negative"
  case _ => "positive"

@main def run(): Unit =
  val b: Byte = 127
  val s: Short = -32768
  val f = 1.5f
  println(b + 1)
  println((b + 1).toByte)
  println(s - 1)
  println((s - 1).toShort)
  println(b * s)
  println(b < s)
  println(b == 127)
  println(-b)
  println(~b)
  println(b.toString + s.toString)
  println(f)
  println(f * 2)
  println(f + 1)
  println(1.0f / 3)
  println(0.1f + 0.2f)
  println(0.1f + 0.2f == 0.3f)
  println(1.1f * 1.1f)
  println(16777216f + 1f)
  println(f.toDouble)
  println(0.1f.toDouble)
  println((f * 3).toInt)
  println(3.7f.toByte)
  println(-3.7f.toShort)
  println(300.toByte)
  println(70000.toShort)
  println(1e10f.toInt)
  println(1e10.toFloat)
  println('a'.toByte)
  println((65: Short).toChar)
  println(1.5f.toLong)
  println(Long.MaxValue.toFloat)
  println(Int.MaxValue.toFloat)
  println(-2.5f)
  println(2.5f % 1)
  println(2.0f / 0)
  println(Float.NaN)
  println(Float.MaxValue)
  println(Float.MinValue)
  println(Byte.MaxValue)
  println(Byte.MinValue)
  println(Short.MaxValue)
  println(Short.MinValue)
  val bytes = Array[Byte](1, 2, 3, 100, 100, 100)
  println(bytes.toList)
  println(checksum(bytes))
  println(List[Byte](3, 1, 2).sorted)
  println(List[Short](300, -1, 7).max)
  println(List(1.5f, 2.5f).sum)
  println(List(2.5f, 1.5f, 3.0f).sorted)
  println(List[Byte](1, 2, 3).sum)
  val p = Packet(7, 1000, 0.25f)
  println(p)
  println(calibrate(p))
  println(p.copy(gain = 2f))
  println(p == Packet(7, 1000, 0.25f))
  val anyf: Any = 1.5f
  println(anyf == 1.5)
  println(anyf == 1.5f)
  println(anyf.isInstanceOf[Float])
  val anyb: Any = b
  println(anyb == 127)
  println(anyb.isInstanceOf[Byte])
  println(describe(b))
  println(describe(s))
  println(describe(f))
  println(describe(1))
  println(sign(0f))
  println(sign(-1f))
  println(sign(f))
  println(s"$f $b $s")
  println("" + f + b)
  println("1.5".toFloat)
  println("12".toByte)
  println("12".toShort)
  println(math.max(1.5f, 2))
  println(math.abs(-2.5f))
  var x: Byte = 1
  x = (x + 1).toByte
  println(x)
  println(1.5f == 1.5)
  println(0.1f == 0.1)
  println(1f == 1)
  println(b.toFloat / 2)
  println(s.toLong * 2)
  val bb: Byte = -1
  println(bb & 0xff)
  println(bb.toChar.toInt)
  println(bb >> 1)
  println(bb >>> 28)
  val widened: Double = f
  println(widened)
  val fromInt: Float = 3
  println(fromInt)
  val fromChar: Float = 'a'
  println(fromChar)
  val longOfShort: Long = s
  println(longOfShort)
  println(Some(1.5f).map(_ * 2))
  println(if b > 0 then 1f else 2.5f)
