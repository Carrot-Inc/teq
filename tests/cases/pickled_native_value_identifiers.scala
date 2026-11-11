trait Evidence:
  def value: Int
given provided: Evidence with
  def value: Int = 7
val marker: Int = 2
object Registry:
  val selected: Int = 11
class Holder(val value: Int)
object Main:
  def imported(holder: Holder): Int =
    import holder.value
    value
  def main(args: Array[String]): Unit =
    println(summon[Evidence].value)
    println(marker + marker)
    println(Registry.selected)
    println(imported(new Holder(13)))
    println(scala.math.Pi)
