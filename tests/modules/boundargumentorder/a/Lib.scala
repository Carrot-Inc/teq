package bao

// An object whose loading shows: the effect is the default of a trait parameter, which the object's own class
// does not write.
trait WithNeither(x: Int = { println("  Bare loaded"); 1 })
object Bare extends WithNeither
def effect(tag: String): Int = { println("  effect " + tag); 1 }
def pair(a: Any, b: Any): Unit = println("  pair called")
