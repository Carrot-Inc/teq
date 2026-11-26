// A given inside a given object is found through the implicit scope of the object's singleton type.
trait T
trait Box[A] { def n: Int }
object O { given G: T with { given b: Box[G.type] with { def n = 42 } } }
object Main { def main(args: Array[String]): Unit = println(summon[Box[O.G.type]].n) }
