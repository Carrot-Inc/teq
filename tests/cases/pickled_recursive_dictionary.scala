trait Evidence:
  def value: Int
object Evidence:
  given evidence(using again: => Evidence): Evidence with
    def value: Int = 17
object Snapshot:
  def read: Int = summon[Evidence].value
object Main:
  def main(args: Array[String]): Unit = println(Snapshot.read)
