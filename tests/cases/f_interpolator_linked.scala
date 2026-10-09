// `f"..."` under every std: scala-library's `StringContext.f` is a macro scalac expands into a
// `format`, which teq's std stands in for there.
@main def run(): Unit =
  val scaled = 12.345
  val unit = "kB"
  println(f"$scaled%.1f $unit")
  println(f"${3.14159}%.2f and ${"x"}")
  println(f"${42}%05d")
  println(f"plain $unit and ${7}")
  println(f"100%% of ${"it"}%s\ttab")
