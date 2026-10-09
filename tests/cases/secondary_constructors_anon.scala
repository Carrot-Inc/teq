class Named(val name: String, val p: Int):
  println(s"named $name")
  def this(name: String) = { this(name, 1); println(s"one-arg $name") }
  def describe: String = s"$name at $p"

@main def m(): Unit =
  val anon = new Named("anon") { override def describe = s"anonymous $name" }
  println(anon.describe)
  val plain = new Named("x", 2) { override def describe = s"plain $name" }
  println(plain.describe)
