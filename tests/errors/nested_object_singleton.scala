// expect: 9:23: error: type mismatch: found (b.R : R), required (a.R : R)
class O:
  object R

object Main:
  def main(args: Array[String]): Unit =
    val a = O()
    val b = O()
    val r: a.R.type = b.R
    println(r)
