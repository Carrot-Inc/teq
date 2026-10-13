// expect: 15:13: error: Reference to constructor proxy for class C in class Inner
// expect: shadows outer reference to object C in object Source
// expect: 1 error found
// An accessible object a wildcard import brings is shadowed by an inner class's constructor proxy
// where it has an `apply`, private or not (`Typer.checkNotShadowed`'s
// `member(nme.apply).hasAltWith`, E177); an inaccessible binding is none
// (`cases/proxy_private_import`).
object Source:
  object C:
    private def apply(x: String): String = x
object Use:
  import Source.*
  class Inner:
    class C(val x: Int)
    val c = C(1)
