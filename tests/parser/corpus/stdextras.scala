//> using platform js
@main def run(): Unit =
  println((0L until 4L).toList)
  println((1L to 3L).map(_ * 10L))
  var total = 0L
  for i <- 0L until 5L do total += i
  println(total)
  println((5L until 5L).isEmpty)
  println(Math.min(3L, 7L))
  println(Math.max(160, Math.min(384, 200)))
  println(Math.abs(-2.5))
  println(Math.sqrt(81.0))
  println(Math.round(2.5))
  println(Math.round(-2.5))
  println(Math.floor(-1.5))
  println("  text  ".stripTrailing + "|")
  println("  text  ".stripLeading + "|")
  println("abc" ++ "..")
  println("hello".capitalize)
  val pages = 23L / 5L + Math.min(23L % 5L, 1L)
  println(pages)
