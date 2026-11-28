class Values(val items: Array[Int]) extends AnyVal

object Main:
  def values(): Values = new Values(Array(2, 5))

  def main(args: Array[String]): Unit =
    for n <- values()
      .items do println(n)
