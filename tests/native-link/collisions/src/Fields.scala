package collide.fields
class Record(var Aa: Int, var BB: Int, var AaAa: Int, var AaBB: Int, var BBAa: Int, var BBBB: Int)
class Wider(v0: Int, v1: Int, v2: Int, v3: Int, v4: Int, v5: Int) extends Record(v0, v1, v2, v3, v4, v5) {
  var extra: Int = 0
}
object Main {
  def main(args: Array[String]): Unit = {
    val r: Record = if (args.length == 0) new Record(0, 1, 2, 3, 4, 5) else new Wider(40, 41, 42, 43, 44, 45)
    r.BB = r.BB + 0
    r.AaAa = r.AaAa + 1
    r.AaBB = r.AaBB + 2
    later(r)
  }
  def later(r: Record): Unit = {
    println(r.BBBB)
    println(r.BBAa)
    println(r.Aa)
  }
}
