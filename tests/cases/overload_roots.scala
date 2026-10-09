//> using scala 3.8.4
trait Named:
  def describe(prefix: String): String = prefix + " named"
trait Tagged:
  def describe(level: Int): String = "tagged " + level
class Plain extends Named:
  override def describe(prefix: String): String = prefix + " plain"
class Both extends Named, Tagged:
  override def describe(prefix: String): String = prefix + " both"
class Deep extends Named, Tagged:
  override def describe(level: Int): String = "deep " + level

trait Codec:
  def encode(x: Int): String = "i" + x
  def encode(x: String): String = "s" + x
trait Sink:
  def encode(x: Int): String
class Wire extends Codec, Sink:
  override def encode(x: Int): String = "wire" + x

def viaNamed(n: Named): String = n.describe("n")
def viaTagged(t: Tagged): String = t.describe(1)
def viaSink(s: Sink): String = s.encode(5)
def viaCodec(c: Codec): String = c.encode(6) + c.encode("x")

@main def run(): Unit =
  println(viaNamed(Plain()))
  println(viaNamed(Both()))
  println(viaTagged(Both()))
  println(viaTagged(Deep()))
  println(viaNamed(Deep()))
  println(Deep().describe(2) + Deep().describe("d"))
  println(viaSink(Wire()))
  println(viaCodec(Wire()))
