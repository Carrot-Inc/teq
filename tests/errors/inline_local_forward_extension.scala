// expect: 10:15: error: forward reference to source (defined on line 11) extends over the definition of first (on line 10)
// expect: 1 error found
// A local inline method called before its definition, whose body reads an extension method an
// import brings from a val the block defines between the two (`1.bump` through `import
// source.bump`): the body is checked with the import entered, `source` standing for itself,
// and reads it, scalac 3.8.4's E039.
class Ops:
  extension (x: Int) def bump: Int = x + 1
def host(): Int =
  val first = g()
  val source = new Ops
  import source.bump
  inline def g(): Int = 1.bump
  first
@main def main(): Unit = println(host())
