// The JDK's answers to the contract fixtures of bench/survey/README.md, which the unit tests hold teq's own
// implementations to (a dependency or a shared implementation that replaces one has to give the same): run on the
// JDK the suites run on, by scalac 3.8.4,
//
//   scala-cli run -S 3.8.4 --jvm system --server=false bench/survey/oracle.scala
//
// One line per answer, `name=value`. deflate: java.util.zip.Deflater (the JDK's bundled zlib) at each level
// over each input, raw and with zlib's wrapper, as the length and the CRC-32 of the output, and the bytes of the
// one the design pass's probe distinguished miniz_oxide by; regex: java.util.regex over the inputs where
// another engine answers otherwise (a back reference, an index past a supplementary character, ASCII and
// Unicode case folding, a search from inside a surrogate pair); crc32: java.util.zip.CRC32's check value.
import java.util.regex.Pattern
import java.util.zip.{CRC32, Deflater}

def crc(b: Array[Byte]): String =
  val c = new CRC32
  c.update(b)
  f"${c.getValue}%08x"

def deflated(input: Array[Byte], level: Int, wrap: Boolean): Array[Byte] =
  val d = new Deflater(level, !wrap)
  d.setInput(input)
  d.finish()
  val out = new java.io.ByteArrayOutputStream
  val buf = new Array[Byte](65536)
  while !d.finished() do out.write(buf, 0, d.deflate(buf))
  d.end()
  out.toByteArray

def hex(b: Array[Byte]): String = b.map(x => f"${x & 255}%02x").mkString

@main def oracle(): Unit =
  val inputs = Vector(
    "short" -> "hello hello hello hello".getBytes("UTF-8"),
    "repeated" -> ("abcde" * 1000).getBytes("UTF-8"),
    "ramp" -> Array.tabulate(8192)(i => (i % 251).toByte),
    "zeros" -> new Array[Byte](10000)
  )
  for (name, input) <- inputs; level <- Vector(0, 1, 6, 9); wrap <- Vector(false, true) do
    val out = deflated(input, level, wrap)
    println(s"deflate.$name.$level.${if wrap then "zlib" else "raw"}=${out.length} ${crc(out)}")
  println("deflate.repeated.6.raw.bytes=" + hex(deflated(inputs(1)._2, 6, false)))
  println("regex.backref=" + Pattern.compile("(a)\\1").matcher("aa").matches())
  val astral = Pattern.compile("b").matcher("😀b")
  astral.find()
  println("regex.astral.start=" + astral.start())
  println("regex.ascii-fold=" + Pattern.compile("(?i)é").matcher("É").matches())
  println("regex.unicode-fold=" + Pattern.compile("(?iu)é").matcher("É").matches())
  val pair = Pattern.compile(".").matcher("😀")
  pair.find(0)
  println(s"regex.pair.whole=${pair.start()} ${pair.end()}")
  pair.find(1)
  println(s"regex.pair.inside=${pair.start()} ${pair.end()}")
  println("crc32.check=" + crc("123456789".getBytes("UTF-8")))
