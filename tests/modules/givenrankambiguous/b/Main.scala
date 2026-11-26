// summon over module a is ambiguous.
object Main { def main(args: Array[String]): Unit = { import O.given; println(summon[T].n) } }
