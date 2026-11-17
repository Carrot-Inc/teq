import scala.language.dynamics
class Plain extends Dynamic:
  def selectDynamic(name: String): String = "dynamic " + name
  def applyDynamic(name: String)(args: Any*): String = "dynamic " + name
extension (p: Plain) inline def extra: String = "extension"
extension (p: Plain) inline def marked(n: Int): String = "extension " + n
class Sub extends Plain:
  def uses(): Unit =
    println(super.extra)
    println(super.marked(1))
    println(super.nn)
    println(super.other)
    println(super.ensuring(true))
// On `super` too an extension method that gives the name comes before the dynamic method (scalac calls the
// extension: `Typer.typedSelectWithAdapt` tries `tryExtensionOrConversion` first). That call through `super` is not
// made: the name is refused as it is for a class that does not derive from `scala.Dynamic`, and never sent to the
// dynamic method. A name nothing gives, and one a conversion would give, go to the dynamic method as under scalac.
// expect: 9:13: error: value extra is not a member of the parents of class Sub
// expect: 10:13: error: value marked is not a member of the parents of class Sub
// expect: 11:13: error: value nn is not a member of the parents of class Sub
// expect: 3 errors found
// absent: other
// absent: ensuring
