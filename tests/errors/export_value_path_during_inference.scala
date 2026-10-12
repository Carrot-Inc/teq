// expect: export_value_path_during_inference.scala:13:10: error: cannot resolve export: foo is not an object or a package
// expect: 1 error found
// absent: recursive value
// The export table a type lookup builds while a val's type is inferred leaves a clause through a
// value to the value forwarders: `Foo` in `foo`'s initializer is looked up in `Test`, whose
// `export foo.pin.*` would widen `foo` while it is inferred. dotty's class completer sees `Foo`
// before the forwarders; only the export is in error, `foo.pin` needing reflection.
trait Foo extends reflect.Selectable
object Test:
  val foo = new Foo:
    object pin:
      val x = 1
  export foo.pin.*

object OK:
  object Foo:
    object pin:
      val x = 1
  export Foo.pin.*
