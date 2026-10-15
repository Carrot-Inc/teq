// A local class's constructor defaults are getters of its local companion, the written one or the one
// `Desugar.classDef` makes for them (`companionDefs`), which captures what they read; a creation selects
// the getter on that companion (`Applications.findDefaultGetter`), so a written companion's initialiser
// runs at the default's place, after the arguments before it. A class nested in a local class has its
// getters on its companion of each enclosing instance, which captures the block's locals they read.
object Main {
  def written(n: Int): Int = {
    var state = 0
    class Item(val a: Int, val x: Int = { println("default"); n + state })
    object Item { println("companion"); state = 10 }
    println("before")
    val i = new Item({ println("argument"); 1 })
    println(i.x)
    new Item(2, 3).x
  }
  def made(n: Int): Int = {
    var reads = 0
    class Item(val x: Int = { reads += 1; if (n == 0) 1 else made(n - 1) + n })(val y: Int = x + reads)
    val i = new Item()()
    println(s"$n:${i.x}:${i.y}:$reads")
    i.y
  }
  def nested(n: Int): Int = {
    var m = 1
    class O {
      class K(val x: Int = n + m, val y: String = "k")
    }
    val o = new O
    m = 2
    new o.K().x + new o.K(y = "z").x
  }
  def caseClass(n: Int): Int = {
    case class P(x: Int = n * 2)
    val a = P()
    a.x + new P().x
  }
  def main(args: Array[String]): Unit = {
    println(written(3))
    println(made(3))
    println(nested(4))
    println(caseClass(5))
  }
}
