// A type parameter whose bound is another file's alias of a refinement conforms to the refinement written out, and
// a class of this file passes its higher-kinded parameter on to the other file's class with the same bound.
import scala.reflect.Selectable.reflectiveSelectable

def keep[T <: Selfish](t: T): AnyRef { def self: Int } = t
class Narrow[G[X] <: AnyRef { def self: Int }] extends Wide[G]

@main def main(): Unit = println(s"${keep(new Knot).self} ${new Narrow[Fixed].tag}")
