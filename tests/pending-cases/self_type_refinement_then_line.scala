// A self type with a refinement, the template's members on the lines after its arrow: the arrow opens no
// block of its own. scalac prints p.
trait C { def p: String }
trait A { self: C { def p: String } =>
  def f: String = p
}
@main def main(): Unit = println((new A with C { def p = "p" }).f)
