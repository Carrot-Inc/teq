// A class of a given object inherits a trait whose outer instance is another object.
class Host(val n: Int) { trait R { def get = n } }
object H extends Host(42)
trait T
object O { given G: T with { class C extends H.R } }
object Main { def main(args: Array[String]): Unit = println(new O.G.C().get) }
