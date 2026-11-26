// summon over module a chooses the given object over the alias of a subtype.
object Main { def main(args: Array[String]): Unit = { import O.given; println(summon[T].n) } }
