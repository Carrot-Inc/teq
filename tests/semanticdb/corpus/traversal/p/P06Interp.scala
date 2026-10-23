package p06
object Interps:
  val n = 1
  val s1 = s"a $n b ${n + 1}"
  val s2 = f"$n%d"
  val s3 = raw"x$n"
  extension (sc: StringContext) def my(args: Any*): String = ""
  val s4 = my"q$n"
