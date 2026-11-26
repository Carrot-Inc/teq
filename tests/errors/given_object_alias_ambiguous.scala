// A given object and a given alias of the same type in one object are ambiguous for summon.
// expect: 5:88: error: Ambiguous given instances: both object G in object O and given instance a in object O match type T of parameter x of method summon in object Predef
trait T { def n: Int }
object O { given G: T with { def n = 1 }; given a: T = new T { def n = 2 } }
object Main { def main(args: Array[String]): Unit = { import O.given; println(summon[T].n) } }
