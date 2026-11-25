object Shapes {
  class P(val _1: Int) extends Product {
    def canEqual(x: Any): Boolean = true
    def productArity: Int = 1
    def productElement(n: Int): Any = _1
  }
  object NullP { def unapply(n: Int): P = { println("null.unapply"); null } }
  class G {
    def _1: Int = { println("_1"); 2 }
    def _2: Int = { println("_2"); 3 }
  }
  class R {
    def isEmpty: Boolean = { println("isEmpty"); false }
    def get: G = { println("get"); new G }
  }
  object E { def unapply(n: Int): R = { println("get.unapply"); new R } }
  class S {
    def lengthCompare(n: Int): Int = { println("lengthCompare " + n); 2 - n }
    def apply(n: Int): Int = { println("apply " + n); n + 2 }
    def drop(n: Int): Seq[Int] = { println("drop " + n); Seq(2,3).drop(n) }
    def toSeq: Seq[Int] = { println("toSeq"); Seq(2,3) }
    def isEmpty: Boolean = { println("DECOY isEmpty"); true }
    def get: S = { println("DECOY get"); this }
  }
  object Q { def unapplySeq(n: Int): S = { println("seq.unapply"); new S } }
  class PS extends Product {
    def _1: Int = { println("prefix"); 2 }
    def _2: S = { println("sequence"); new S }
    def canEqual(x: Any): Boolean = true
    def productArity: Int = 2
    def productElement(n: Int): Any = if n == 0 then _1 else _2
  }
  object PQ { def unapplySeq(n: Int): PS = { println("prodseq.unapply"); new PS } }
  def main(args: Array[String]): Unit = {
    println("-- null pure wildcard")
    try println(1 match { case NullP(_) => 1; case _ => 0 })
    catch { case _: NullPointerException => println("NPE") }
    println("-- eager get selectors")
    println(1 match { case E(1, _) => 1; case _ => 0 })
    println("-- sequence failed prefix")
    println(1 match { case Q(1, xs*) => 1; case _ => 0 })
    println("-- sequence star")
    println(1 match { case Q(xs*) => xs.size; case _ => 0 })
    println("-- product sequence failed prefix")
    println(1 match { case PQ(1, xs*) => 1; case _ => 0 })
  }
}
