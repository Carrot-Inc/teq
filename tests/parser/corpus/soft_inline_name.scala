object Opts:
  def set(name: String, v: Any): Unit = println(s"$name=$v")
  def apply(block: String = "start", inline: String = "nearest"): Unit =
    set("block", block)
    set("inline", inline)
  def twice(inline: Int): Int = 2 * inline
  val inline = 3
@main def run(): Unit =
  Opts()
  println(Opts.twice(4))
  println(Opts.inline * 2)
  inline val k = 5
  println(k)
