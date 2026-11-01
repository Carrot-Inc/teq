// jars: fixtures scala-library
// expect: value s overrides nothing
// A library case class's private accessor (tests/tasty/src/paramqual.scala's `case class PqCase(private val s: Int)`):
// TASTy keeps `PRIVATE` beside `CASEaccessor` (`TreeUnpickler.readModifiers`), so a subclass's `override val s`
// overrides nothing (scalac 3.8.4's E037). teq's loader makes a case accessor a field and drops its `PRIVATE`
// (loader/mod.rs `enter_loaded_ctor`, `is_field`), so the subclass overrides it and is accepted: the loaded symbols'
// flags, pinned to the type-representation push.
class PqCaseSub(override val s: Int) extends fix.shapes.PqCase(s)
