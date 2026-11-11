trait Counter:
  def value: Int
trait WithHiddenGivens:
  private given counter: Counter with
    def value: Int = 17
  private given label: String = "label"
  private given answer(using enabled: Boolean): Int = if enabled then 37 else 0
  def total: Int =
    given Boolean = true
    summon[Counter].value + summon[String].length + summon[Int]
@main def run(): Unit = println((new WithHiddenGivens {}).total)
