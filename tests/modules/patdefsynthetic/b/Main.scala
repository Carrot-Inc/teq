package pds

object Main {
  def main(args: Array[String]): Unit = {
    println(Inspect.check(Held.sum(Held.flag)))
    println(Inspect.fields[Held.type])
    println(Held.x + Held.u)
  }
}
