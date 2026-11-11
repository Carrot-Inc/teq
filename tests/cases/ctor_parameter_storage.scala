class SeededCell(seed: Int):
  private var value = seed
  def current: Int = value
class PrivateValue(private val start: Int):
  val current = start + 1
class ReadLater(private val kept: Int):
  def current = kept
class MutableValue(private var count: Int):
  count += 1
  val current = count
class OtherReceiver(private val number: Int, other: OtherReceiver):
  val current = if other == null then number else other.number
class LazyValue(seed: Int):
  lazy val current = seed
class CapturedValue(seed: Int):
  val current = () => seed
class PublicValue(val visible: Int)
@main def run(): Unit =
  println(new SeededCell(7).current)
  println(new PrivateValue(7).current)
  println(new ReadLater(9).current)
  println(new MutableValue(9).current)
  val first = new OtherReceiver(11, null)
  println(new OtherReceiver(0, first).current)
  println(new LazyValue(12).current)
  println(new CapturedValue(13).current())
  println(new PublicValue(14).visible)
