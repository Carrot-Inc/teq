// A class of a parameterless given reads the given's val and names the given's own instance (`G.this`).
trait T
given G: T with {
  val base = 21
  class K { def get: Int = base; def owner: AnyRef = G.this }
}
object Main { def main(args: Array[String]): Unit = {
  val k = new G.K
  println(k.get)
  println(k.owner eq G)
}}
