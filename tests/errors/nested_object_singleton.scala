// expect: 9:23: error: Found:    (b.R : R)⏎Required: (a.R : R)
class O:
  object R

object Main:
  def main(args: Array[String]): Unit =
    val a = O()
    val b = O()
    val r: a.R.type = b.R
    println(r)
