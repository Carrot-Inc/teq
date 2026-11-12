// An object nested in a top-level given object is made once and is the same object at every read. dotty makes it a
// static module (the given object is a module class, a static owner); teq's static placement admits objects only as
// static owners and holds it as a lazy member of the given's module class, thread-safe (a queued item), so the
// values are the same and the class files differ.
trait Registry:
  def name: String

given registry: Registry with
  def name = "registry"
  object Inner:
    var made = 0
    made += 1
    val tag = "inner of " + name

object Main:
  def main(args: Array[String]): Unit =
    val r = summon[Registry]
    println(registry.Inner.tag)
    println(registry.Inner eq registry.Inner)
    println(registry.Inner.made)
    println(r.name)
