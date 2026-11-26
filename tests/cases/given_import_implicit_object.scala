// An explicit import of an implicit object makes it a candidate of summon.
trait T { def n: Int }
object O { implicit object i extends T { def n = 42 } }
object Main { def main(args: Array[String]): Unit = { import O.i; println(summon[T].n) } }
