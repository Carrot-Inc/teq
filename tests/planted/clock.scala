// The planted slowdown's case (tests/planted.sh): the clock read in code, bare and qualified by java.lang.,
// and its name in strings, an interpolation, a character's neighbour and comments, which stay text.
object PlantedClock:
  /* System.nanoTime() in a /* nested */ comment */
  def main(args: Array[String]): Unit =
    println("System.nanoTime() is the clock")
    println(s"read ${"System.nanoTime()"} through it")
    val q = '"'
    println("" + q + "java.lang.System.nanoTime()" + q)
    val a = System.nanoTime() // System.nanoTime() in a line comment
    var x = 0L
    var i = 0
    while i < 200000 do
      x += i % 7
      i += 1
    val b = java.lang.System.nanoTime()
    val c = _root_.java.lang.System.nanoTime()
    println("ordered " + (a <= b && b <= c))
    println("sum " + x)
